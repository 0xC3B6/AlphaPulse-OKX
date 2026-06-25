use crate::domain::Candle;
use serde::Deserialize;

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
            http: reqwest::Client::new(),
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
}

impl Default for OkxRestClient {
    fn default() -> Self {
        Self::new()
    }
}
