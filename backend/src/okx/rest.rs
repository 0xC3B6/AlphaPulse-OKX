use crate::domain::{Candle, Timeframe};
use serde::Deserialize;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub struct TickerRow {
    pub inst_id: String,
    pub last: f64,
    pub quote_volume_24h: f64,
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
    vol_ccy24h: String,
    ts: String,
}

pub fn parse_tickers(json: &str) -> anyhow::Result<Vec<TickerRow>> {
    let response: OkxResponse<Vec<RawTicker>> = serde_json::from_str(json)?;
    response
        .data
        .into_iter()
        .map(|row| {
            Ok(TickerRow {
                inst_id: row.inst_id,
                last: row.last.parse()?,
                quote_volume_24h: row.vol_ccy24h.parse()?,
                ts_ms: row.ts.parse()?,
            })
        })
        .collect()
}

pub fn parse_candles(json: &str) -> anyhow::Result<Vec<Candle>> {
    let response: OkxResponse<Vec<Vec<String>>> = serde_json::from_str(json)?;
    response
        .data
        .into_iter()
        .map(|row| {
            anyhow::ensure!(row.len() >= 6, "OKX candle row has fewer than 6 fields");
            Ok(Candle {
                ts_ms: row[0].parse()?,
                open: row[1].parse()?,
                high: row[2].parse()?,
                low: row[3].parse()?,
                close: row[4].parse()?,
                volume: row[5].parse()?,
            })
        })
        .collect()
}

#[derive(Clone)]
pub struct OkxRestClient {
    http: reqwest::Client,
    base_url: String,
}

impl OkxRestClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .expect("OKX REST client configuration should be valid"),
            base_url: "https://www.okx.com".to_string(),
        }
    }

    pub async fn get_json(&self, path: &str) -> anyhow::Result<String> {
        let url = format!("{}{}", self.base_url, path);
        Ok(self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?)
    }

    pub async fn fetch_swap_tickers(&self) -> anyhow::Result<Vec<TickerRow>> {
        let json = self
            .get_json("/api/v5/market/tickers?instType=SWAP")
            .await?;
        parse_tickers(&json)
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
}

impl Default for OkxRestClient {
    fn default() -> Self {
        Self::new()
    }
}
