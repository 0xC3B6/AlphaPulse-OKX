use std::time::Duration;

use chrono::{NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::Candle;

const AHR999_MA_PERIOD: usize = 200;
const BTC_GENESIS_DATE_MS: i64 = 1_230_940_800_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalMetricStatus {
    pub id: String,
    pub name: String,
    pub status: String,
    pub note: String,
    pub value: Option<f64>,
    pub date: Option<String>,
    pub source: Option<String>,
    pub zone: Option<String>,
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ahr999History {
    pub source: String,
    pub points: Vec<Ahr999HistoryPoint>,
    pub bands: Vec<Ahr999RangeBand>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ahr999HistoryPoint {
    pub ts_ms: i64,
    pub date: String,
    pub value: f64,
    pub btc_price: f64,
    pub gma200: f64,
    pub model_price: f64,
    pub zone: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ahr999RangeBand {
    pub id: String,
    pub label: String,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub days: usize,
    pub recommendation: String,
}

#[derive(Clone)]
pub struct CoinglassValuationClient {
    http: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl CoinglassValuationClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Coinglass HTTP client configuration should be valid"),
            base_url: "https://open-api-v4.coinglass.com".to_string(),
            api_key,
        }
    }

    pub async fn fetch_metrics(&self) -> Vec<ExternalMetricStatus> {
        let Some(api_key) = self.api_key.as_deref() else {
            return valuation_metrics_unavailable("coinglass api key missing");
        };

        let ahr999 = match self.fetch_ahr999(api_key).await {
            Ok(metric) => metric,
            Err(error) => {
                return valuation_metrics_unavailable(&format!(
                    "coinglass ahr999 unavailable: {error}"
                ));
            }
        };

        vec![ahr999, mvrv_z_pending()]
    }

    async fn fetch_ahr999(&self, api_key: &str) -> anyhow::Result<ExternalMetricStatus> {
        let url = format!("{}/api/index/ahr999", self.base_url);
        let json = self
            .http
            .get(url)
            .header("CG-API-KEY", api_key)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        parse_coinglass_ahr999(&json)
    }
}

#[derive(Debug, Deserialize)]
struct CoinglassAhr999Response {
    code: String,
    msg: String,
    #[serde(default)]
    data: Vec<CoinglassAhr999Row>,
}

#[derive(Debug, Deserialize)]
struct CoinglassAhr999Row {
    date_string: String,
    average_price: f64,
    ahr999_value: f64,
    current_value: f64,
}

pub fn parse_coinglass_ahr999(json: &str) -> anyhow::Result<ExternalMetricStatus> {
    let response: CoinglassAhr999Response = serde_json::from_str(json)?;
    anyhow::ensure!(
        response.code == "0",
        "coinglass returned {}: {}",
        response.code,
        response.msg
    );
    let latest = response
        .data
        .iter()
        .rev()
        .find(|row| {
            row.ahr999_value.is_finite()
                && row.current_value.is_finite()
                && row.average_price.is_finite()
                && !row.date_string.trim().is_empty()
        })
        .ok_or_else(|| anyhow::anyhow!("coinglass ahr999 response has no usable rows"))?;

    Ok(ExternalMetricStatus {
        id: "ahr999".to_string(),
        name: "AHR999".to_string(),
        status: "available".to_string(),
        note: format!(
            "current={:.2} average={:.2}",
            latest.current_value, latest.average_price
        ),
        value: Some(latest.ahr999_value),
        date: Some(latest.date_string.clone()),
        source: Some("coinglass".to_string()),
        zone: Some(ahr999_zone(latest.ahr999_value).to_string()),
        updated_at_ms: parse_coinglass_date_ms(&latest.date_string),
    })
}

pub fn self_calculated_ahr999(candles: &[Candle]) -> anyhow::Result<ExternalMetricStatus> {
    let history = self_calculated_ahr999_history(candles)?;
    let latest = history
        .points
        .last()
        .ok_or_else(|| anyhow::anyhow!("AHR999 history has no usable points"))?;

    Ok(ExternalMetricStatus {
        id: "ahr999".to_string(),
        name: "AHR999".to_string(),
        status: "available".to_string(),
        note: format!(
            "close={:.2} gma200={:.2} model={:.2}",
            latest.btc_price, latest.gma200, latest.model_price
        ),
        value: Some(latest.value),
        date: Some(latest.date.clone()),
        source: Some(history.source),
        zone: Some(latest.zone.clone()),
        updated_at_ms: Some(latest.ts_ms),
    })
}

pub fn self_calculated_ahr999_history(candles: &[Candle]) -> anyhow::Result<Ahr999History> {
    anyhow::ensure!(
        candles.len() >= AHR999_MA_PERIOD,
        "at least {AHR999_MA_PERIOD} daily BTC candles are required for AHR999"
    );

    let mut points = Vec::new();
    for index in (AHR999_MA_PERIOD - 1)..candles.len() {
        points.push(ahr999_point_for_window(
            &candles[(index + 1 - AHR999_MA_PERIOD)..=index],
        )?);
    }

    Ok(Ahr999History {
        source: "self_calculated_okx".to_string(),
        bands: ahr999_bands(&points),
        points,
    })
}

fn ahr999_point_for_window(window: &[Candle]) -> anyhow::Result<Ahr999HistoryPoint> {
    let latest = window
        .last()
        .ok_or_else(|| anyhow::anyhow!("daily BTC candles are required for AHR999"))?;
    anyhow::ensure!(
        latest.close > 0.0 && latest.close.is_finite(),
        "latest BTC close is invalid for AHR999"
    );

    let gma200_log_sum = window.iter().try_fold(0.0, |sum, candle| {
        anyhow::ensure!(
            candle.close > 0.0 && candle.close.is_finite(),
            "BTC 200D geometric average input is invalid for AHR999"
        );
        Ok::<_, anyhow::Error>(sum + candle.close.ln())
    })?;
    let gma200 = (gma200_log_sum / window.len() as f64).exp();
    anyhow::ensure!(
        gma200 > 0.0 && gma200.is_finite(),
        "BTC 200D geometric average is invalid for AHR999"
    );

    let days_since_genesis = ((latest.ts_ms - BTC_GENESIS_DATE_MS) / 86_400_000).max(1) as f64;
    let model_price = 10_f64.powf(5.84 * days_since_genesis.log10() - 17.01);
    anyhow::ensure!(
        model_price > 0.0 && model_price.is_finite(),
        "BTC exponential model price is invalid for AHR999"
    );

    let value = (latest.close / gma200) * (latest.close / model_price);

    Ok(Ahr999HistoryPoint {
        ts_ms: latest.ts_ms,
        date: format_candle_date(latest.ts_ms).unwrap_or_else(|| latest.ts_ms.to_string()),
        value,
        btc_price: latest.close,
        gma200,
        model_price,
        zone: ahr999_zone(value).to_string(),
    })
}

fn ahr999_bands(points: &[Ahr999HistoryPoint]) -> Vec<Ahr999RangeBand> {
    ahr999_band_definitions()
        .into_iter()
        .map(|mut band| {
            band.days = points
                .iter()
                .filter(|point| value_in_band(point.value, band.lower, band.upper))
                .count();
            band
        })
        .collect()
}

fn ahr999_band_definitions() -> Vec<Ahr999RangeBand> {
    vec![
        Ahr999RangeBand {
            id: "deep_value".to_string(),
            label: "AHR999 < 0.45".to_string(),
            lower: None,
            upper: Some(0.45),
            days: 0,
            recommendation:
                "deep value zone; spot accumulation or very low leverage only, avoid forced exits"
                    .to_string(),
        },
        Ahr999RangeBand {
            id: "accumulation".to_string(),
            label: "0.45 <= AHR999 < 1.2".to_string(),
            lower: Some(0.45),
            upper: Some(1.2),
            days: 0,
            recommendation:
                "accumulation zone; staged entries can be scored higher after trend confirmation"
                    .to_string(),
        },
        Ahr999RangeBand {
            id: "neutral".to_string(),
            label: "1.2 <= AHR999 < 5".to_string(),
            lower: Some(1.2),
            upper: Some(5.0),
            days: 0,
            recommendation:
                "neutral trend zone; prefer price structure and risk controls over valuation alone"
                    .to_string(),
        },
        Ahr999RangeBand {
            id: "overheated".to_string(),
            label: "AHR999 >= 5".to_string(),
            lower: Some(5.0),
            upper: None,
            days: 0,
            recommendation:
                "overheated zone; de-risk long exposure and avoid chasing late-cycle breakouts"
                    .to_string(),
        },
    ]
}

fn value_in_band(value: f64, lower: Option<f64>, upper: Option<f64>) -> bool {
    lower.is_none_or(|lower| value >= lower) && upper.is_none_or(|upper| value < upper)
}

pub fn apply_ahr999_fallback(
    metrics: &mut Vec<ExternalMetricStatus>,
    fallback: ExternalMetricStatus,
) {
    if let Some(metric) = metrics
        .iter_mut()
        .find(|metric| metric.id == "ahr999" && metric.status != "available")
    {
        *metric = fallback;
        return;
    }
    if !metrics.iter().any(|metric| metric.id == "ahr999") {
        metrics.insert(0, fallback);
    }
}

pub fn valuation_metrics_unavailable(reason: &str) -> Vec<ExternalMetricStatus> {
    vec![ahr999_unavailable(reason), mvrv_z_pending()]
}

pub fn valuation_metrics_pending() -> Vec<ExternalMetricStatus> {
    vec![
        ExternalMetricStatus {
            id: "ahr999".to_string(),
            name: "AHR999".to_string(),
            status: "data_source_pending".to_string(),
            note: "coinglass valuation provider not loaded".to_string(),
            value: None,
            date: None,
            source: None,
            zone: None,
            updated_at_ms: None,
        },
        mvrv_z_pending(),
    ]
}

fn ahr999_unavailable(reason: &str) -> ExternalMetricStatus {
    ExternalMetricStatus {
        id: "ahr999".to_string(),
        name: "AHR999".to_string(),
        status: "unavailable".to_string(),
        note: reason.to_string(),
        value: None,
        date: None,
        source: Some("coinglass".to_string()),
        zone: None,
        updated_at_ms: None,
    }
}

fn mvrv_z_pending() -> ExternalMetricStatus {
    ExternalMetricStatus {
        id: "mvrv_z".to_string(),
        name: "MVRV-Z Score".to_string(),
        status: "data_source_pending".to_string(),
        note: "coinglass official MVRV-Z API endpoint is not available yet".to_string(),
        value: None,
        date: None,
        source: None,
        zone: None,
        updated_at_ms: None,
    }
}

fn ahr999_zone(value: f64) -> &'static str {
    if value < 0.45 {
        "deep_value"
    } else if value < 1.2 {
        "accumulation"
    } else if value < 5.0 {
        "neutral"
    } else {
        "overheated"
    }
}

fn parse_coinglass_date_ms(value: &str) -> Option<i64> {
    NaiveDate::parse_from_str(value, "%Y/%m/%d")
        .ok()?
        .and_hms_opt(0, 0, 0)?
        .and_utc()
        .timestamp_millis()
        .into()
}

fn format_candle_date(ts_ms: i64) -> Option<String> {
    let dt = Utc.timestamp_millis_opt(ts_ms).single()?;
    Some(dt.format("%Y/%m/%d").to_string())
}
