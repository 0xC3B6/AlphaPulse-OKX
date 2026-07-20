use crate::{
    domain::{Candle, Timeframe},
    universe::InstrumentMetadata,
};
use serde::Deserialize;
use std::time::Duration;

const OKX_REST_TIMEOUT_SECS: u64 = 10;
const OKX_REST_ATTEMPTS: usize = 3;
const OKX_REST_RETRY_DELAY_MS: u64 = 250;
const OKX_CANDLE_PAGE_LIMIT: usize = 300;
const OKX_HISTORY_CANDLE_PAGE_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub struct TickerRow {
    pub inst_id: String,
    pub last: f64,
    pub quote_volume_24h: f64,
    pub change_24h_pct: f64,
    pub amplitude_24h_pct: f64,
    pub ts_ms: i64,
}

#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTicker {
    inst_id: String,
    last: String,
    open24h: Option<String>,
    high24h: Option<String>,
    low24h: Option<String>,
    vol_ccy24h: String,
    ts: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawInstrument {
    inst_id: String,
    state: String,
    list_time: String,
}

pub fn parse_tickers(json: &str) -> anyhow::Result<Vec<TickerRow>> {
    let response: OkxResponse<Vec<RawTicker>> = serde_json::from_str(json)?;
    response
        .data
        .into_iter()
        .map(|row| {
            let last = row.last.parse()?;
            let change_24h_pct = row
                .open24h
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|open| *open > 0.0)
                .map(|open| (last - open) / open)
                .unwrap_or(0.0);
            let amplitude_24h_pct = row
                .high24h
                .as_deref()
                .and_then(|value| value.parse::<f64>().ok())
                .zip(
                    row.low24h
                        .as_deref()
                        .and_then(|value| value.parse::<f64>().ok()),
                )
                .filter(|(high, low)| *low > 0.0 && *high >= *low)
                .map(|(high, low)| (high - low) / low)
                .unwrap_or(0.0);
            Ok(TickerRow {
                inst_id: row.inst_id,
                last,
                quote_volume_24h: row.vol_ccy24h.parse()?,
                change_24h_pct,
                amplitude_24h_pct,
                ts_ms: row.ts.parse()?,
            })
        })
        .collect()
}

pub fn parse_instruments(json: &str) -> anyhow::Result<Vec<InstrumentMetadata>> {
    let response: OkxResponse<Vec<RawInstrument>> = serde_json::from_str(json)?;
    response
        .data
        .into_iter()
        .map(|row| {
            Ok(InstrumentMetadata {
                inst_id: row.inst_id,
                state: row.state,
                list_time_ms: row.list_time.parse()?,
            })
        })
        .collect()
}

pub fn parse_candles(json: &str) -> anyhow::Result<Vec<Candle>> {
    let response: OkxResponse<Vec<Vec<String>>> = serde_json::from_str(json)?;
    let parsed = response
        .data
        .into_iter()
        .map(|row| {
            anyhow::ensure!(row.len() >= 9, "OKX candle row has no confirmation flag");
            anyhow::ensure!(
                row[8] == "0" || row[8] == "1",
                "OKX candle confirmation flag is invalid"
            );
            if row[8] == "0" {
                return Ok(None);
            }
            Ok(Some(Candle {
                ts_ms: row[0].parse()?,
                open: row[1].parse()?,
                high: row[2].parse()?,
                low: row[3].parse()?,
                close: row[4].parse()?,
                volume: row[5].parse()?,
            }))
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(parsed.into_iter().flatten().collect())
}

#[derive(Clone)]
pub struct OkxRestClient {
    http: reqwest::Client,
    base_url: String,
}

impl OkxRestClient {
    pub fn new() -> Self {
        Self::with_base_url("https://www.okx.com")
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(OKX_REST_TIMEOUT_SECS))
                .build()
                .expect("OKX REST client configuration should be valid"),
            base_url: base_url.into(),
        }
    }

    pub async fn get_json(&self, path: &str) -> anyhow::Result<String> {
        let url = format!("{}{}", self.base_url, path);
        let mut last_error = None;
        for attempt in 0..OKX_REST_ATTEMPTS {
            match self
                .http
                .get(&url)
                .send()
                .await
                .and_then(|response| response.error_for_status())
            {
                Ok(response) => return Ok(response.text().await?),
                Err(error) => {
                    let retry = attempt + 1 < OKX_REST_ATTEMPTS && should_retry_okx_error(&error);
                    last_error = Some(error);
                    if retry {
                        tokio::time::sleep(Duration::from_millis(OKX_REST_RETRY_DELAY_MS)).await;
                    } else {
                        break;
                    }
                }
            }
        }
        Err(last_error
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow::anyhow!("OKX request did not run")))
    }

    pub async fn fetch_swap_tickers(&self) -> anyhow::Result<Vec<TickerRow>> {
        let json = self
            .get_json("/api/v5/market/tickers?instType=SWAP")
            .await?;
        parse_tickers(&json)
    }

    pub async fn fetch_swap_instruments(&self) -> anyhow::Result<Vec<InstrumentMetadata>> {
        let json = self
            .get_json("/api/v5/public/instruments?instType=SWAP")
            .await?;
        parse_instruments(&json)
    }

    pub async fn fetch_candles(
        &self,
        inst_id: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> anyhow::Result<Vec<Candle>> {
        let path = format!(
            "/api/v5/market/candles?instId={inst_id}&bar={}&limit={limit}",
            timeframe.okx_bar()
        );
        let json = self.get_json(&path).await?;
        let mut candles = parse_candles(&json)?;
        candles.sort_by_key(|candle| candle.ts_ms);
        Ok(candles)
    }

    pub async fn fetch_candles_with_history(
        &self,
        inst_id: &str,
        timeframe: Timeframe,
        limit: usize,
    ) -> anyhow::Result<Vec<Candle>> {
        let mut candles = self
            .fetch_candles(inst_id, timeframe, limit.min(OKX_CANDLE_PAGE_LIMIT))
            .await?;
        while candles.len() < limit {
            let Some(oldest) = candles.first().map(|candle| candle.ts_ms) else {
                break;
            };
            let page_limit = (limit - candles.len()).min(OKX_HISTORY_CANDLE_PAGE_LIMIT);
            let path = format!(
                "/api/v5/market/history-candles?instId={inst_id}&bar={}&after={oldest}&limit={page_limit}",
                timeframe.okx_bar()
            );
            let Ok(json) = self.get_json(&path).await else {
                break;
            };
            let Ok(mut older) = parse_candles(&json) else {
                break;
            };
            older.retain(|candle| candle.ts_ms < oldest);
            if older.is_empty() {
                break;
            }
            older.sort_by_key(|candle| candle.ts_ms);
            older.extend(candles);
            candles = older;
        }
        candles.sort_by_key(|candle| candle.ts_ms);
        candles.dedup_by_key(|candle| candle.ts_ms);
        if candles.len() > limit {
            candles = candles[(candles.len() - limit)..].to_vec();
        }
        Ok(candles)
    }
}

fn should_retry_okx_error(error: &reqwest::Error) -> bool {
    if error.is_timeout() || error.is_connect() || error.is_request() {
        return true;
    }
    error.status().is_some_and(|status| {
        status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
    })
}

impl Default for OkxRestClient {
    fn default() -> Self {
        Self::new()
    }
}
