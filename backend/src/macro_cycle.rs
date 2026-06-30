use chrono::{Datelike, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{Candle, Timeframe},
    okx::rest::OkxRestClient,
    valuation::{self, Ahr999History, CoinglassValuationClient, ExternalMetricStatus},
};

const BTC_INST_ID: &str = "BTC-USDT-SWAP";
const MA_200W_PERIOD: usize = 200;
const WEEKLY_LIMIT: usize = 260;
const DAILY_LIMIT: usize = 2600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroRegime {
    BullExpansion,
    LateCycleDistribution,
    BearMarket,
    BearMarketRally,
    BottomingAccumulation,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BtcMacroSnapshot {
    pub asset: String,
    pub updated_at_ms: i64,
    pub price: f64,
    pub regime: MacroRegime,
    pub confidence: u8,
    pub summary: String,
    pub cycle: HalvingCycleSnapshot,
    pub trend: MacroTrendSnapshot,
    pub momentum: MacroMomentumSnapshot,
    pub events: Vec<MacroEvent>,
    pub valuation_metrics: Vec<ExternalMetricStatus>,
    pub ahr999_history: Option<Ahr999History>,
    pub analogs: Vec<HistoricalAnalog>,
    pub analog_comparisons: Vec<AnalogComparisonSet>,
    pub trading_bias: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalvingCycleSnapshot {
    pub last_halving_ms: i64,
    pub next_halving_estimate_ms: i64,
    pub days_since_halving: i64,
    pub estimated_cycle_progress_pct: f64,
    pub cycle_year: u8,
    pub cycle_quarter: u8,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroTrendSnapshot {
    pub window_ath: f64,
    pub window_ath_ts_ms: i64,
    pub drawdown_from_window_ath_pct: f64,
    pub ma_200w: Option<f64>,
    pub price_vs_200w_pct: Option<f64>,
    pub weekly_ma200_status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroMomentumSnapshot {
    pub change_30d_pct: Option<f64>,
    pub change_90d_pct: Option<f64>,
    pub change_26w_pct: Option<f64>,
    pub volatility_90d_pct: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MacroEvent {
    pub id: String,
    pub title: String,
    pub event_type: String,
    pub date_ms: i64,
    pub days_to_event: i64,
    pub phase: String,
    pub impact_tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalAnalog {
    pub label: String,
    pub score: u8,
    pub rationale: Vec<String>,
    pub components: Vec<AnalogScoreComponent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogScoreComponent {
    pub label: String,
    pub points: u8,
    pub max_points: u8,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogComparisonSet {
    pub timeframe_days: usize,
    pub current: Option<AnalogPathSummary>,
    pub matches: Vec<AnalogMatch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogPathSummary {
    pub start_ts_ms: i64,
    pub end_ts_ms: i64,
    pub final_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub max_runup_pct: f64,
    pub candles: Vec<AnalogKline>,
    pub path: Vec<AnalogPathPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogKline {
    pub ts_ms: i64,
    pub offset_days: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub index_open: f64,
    pub index_high: f64,
    pub index_low: f64,
    pub index_close: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogPathPoint {
    pub offset_days: i64,
    pub return_pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogMatch {
    pub id: String,
    pub label: String,
    pub score: u8,
    pub start_ts_ms: i64,
    pub end_ts_ms: i64,
    pub final_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub max_runup_pct: f64,
    pub components: Vec<AnalogScoreComponent>,
    pub lookback: AnalogPathSummary,
    pub forward: Option<AnalogPathSummary>,
    pub path: Vec<AnalogPathPoint>,
}

pub async fn fetch_btc_macro_snapshot(
    rest: &OkxRestClient,
    valuation_client: &CoinglassValuationClient,
) -> anyhow::Result<BtcMacroSnapshot> {
    let daily = rest
        .fetch_candles_with_history(BTC_INST_ID, Timeframe::D1, DAILY_LIMIT)
        .await
        .unwrap_or_default();
    let weekly = match rest
        .fetch_candles(BTC_INST_ID, Timeframe::W1, WEEKLY_LIMIT)
        .await
    {
        Ok(weekly) => weekly,
        Err(error) => {
            let fallback = weekly_from_daily(&daily);
            if fallback.is_empty() {
                return Err(error);
            }
            fallback
        }
    };

    let mut snapshot = build_btc_macro_snapshot(&weekly, &daily, Utc::now().timestamp_millis())?;
    snapshot.valuation_metrics = valuation_client.fetch_metrics().await;
    if let Ok(metric) = valuation::self_calculated_ahr999(&daily) {
        valuation::apply_ahr999_fallback(&mut snapshot.valuation_metrics, metric);
    }
    Ok(snapshot)
}

pub fn build_btc_macro_snapshot(
    weekly: &[Candle],
    daily: &[Candle],
    now_ms: i64,
) -> anyhow::Result<BtcMacroSnapshot> {
    anyhow::ensure!(!weekly.is_empty(), "weekly BTC candles are required");
    let price = daily
        .last()
        .or_else(|| weekly.last())
        .map(|candle| candle.close)
        .unwrap_or_default();
    anyhow::ensure!(price > 0.0 && price.is_finite(), "BTC price is unavailable");

    let cycle = halving_cycle(now_ms);
    let trend = macro_trend(weekly, price);
    let momentum = macro_momentum(weekly, daily);
    let events = vec![us_midterm_2026_event(now_ms)];
    let regime = classify_regime(&cycle, &trend, &momentum);
    let confidence = regime_confidence(&trend, &momentum);
    let analogs = historical_analogs(&regime, &cycle, &trend, &momentum);
    let analog_comparisons = historical_analog_comparisons(daily);
    let trading_bias = trading_bias(&regime, &trend, &events);
    let summary = regime_summary(&regime, &cycle, &trend);
    let ahr999_history = valuation::self_calculated_ahr999_history(daily).ok();

    Ok(BtcMacroSnapshot {
        asset: "BTC".to_string(),
        updated_at_ms: now_ms,
        price,
        regime,
        confidence,
        summary,
        cycle,
        trend,
        momentum,
        events,
        valuation_metrics: valuation::valuation_metrics_pending(),
        ahr999_history,
        analogs,
        analog_comparisons,
        trading_bias,
    })
}

fn halving_cycle(now_ms: i64) -> HalvingCycleSnapshot {
    let halvings = [
        utc_ms(2012, 11, 28),
        utc_ms(2016, 7, 9),
        utc_ms(2020, 5, 11),
        utc_ms(2024, 4, 20),
        utc_ms(2028, 4, 20),
    ];
    let last_halving_ms = halvings
        .iter()
        .copied()
        .take_while(|halving| *halving <= now_ms)
        .last()
        .unwrap_or(halvings[0]);
    let next_halving_estimate_ms = halvings
        .iter()
        .copied()
        .find(|halving| *halving > now_ms)
        .unwrap_or_else(|| utc_ms(2032, 4, 20));

    let days_since_halving = days_between(last_halving_ms, now_ms).max(0);
    let cycle_days = days_between(last_halving_ms, next_halving_estimate_ms).max(1);
    let estimated_cycle_progress_pct = days_since_halving as f64 / cycle_days as f64;
    let cycle_year = (days_since_halving / 365 + 1).clamp(1, 4) as u8;
    let cycle_quarter = (days_since_halving / 91 + 1).clamp(1, 16) as u8;
    let phase = match days_since_halving {
        0..=180 => "post_halving_accumulation",
        181..=540 => "expansion_window",
        541..=900 => "late_cycle_distribution_window",
        901..=1200 => "cycle_decay_window",
        _ => "bottoming_window",
    }
    .to_string();

    HalvingCycleSnapshot {
        last_halving_ms,
        next_halving_estimate_ms,
        days_since_halving,
        estimated_cycle_progress_pct,
        cycle_year,
        cycle_quarter,
        phase,
    }
}

fn macro_trend(weekly: &[Candle], price: f64) -> MacroTrendSnapshot {
    let (window_ath, window_ath_ts_ms) = weekly
        .iter()
        .max_by(|left, right| {
            left.high
                .partial_cmp(&right.high)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|candle| (candle.high, candle.ts_ms))
        .unwrap_or((
            price,
            weekly.last().map(|candle| candle.ts_ms).unwrap_or_default(),
        ));
    let drawdown_from_window_ath_pct = if window_ath > 0.0 {
        price / window_ath - 1.0
    } else {
        0.0
    };
    let ma_200w = moving_average(weekly, MA_200W_PERIOD);
    let price_vs_200w_pct = ma_200w.map(|ma| price / ma - 1.0);
    let weekly_ma200_status = match price_vs_200w_pct {
        Some(value) if value <= -0.10 => "below_200w_ma",
        Some(value) if value < 0.05 => "near_200w_ma",
        Some(_) => "above_200w_ma",
        None => "insufficient_history",
    }
    .to_string();

    MacroTrendSnapshot {
        window_ath,
        window_ath_ts_ms,
        drawdown_from_window_ath_pct,
        ma_200w,
        price_vs_200w_pct,
        weekly_ma200_status,
    }
}

fn macro_momentum(weekly: &[Candle], daily: &[Candle]) -> MacroMomentumSnapshot {
    MacroMomentumSnapshot {
        change_30d_pct: change_over_bars(daily, 30),
        change_90d_pct: change_over_bars(daily, 90),
        change_26w_pct: change_over_bars(weekly, 26),
        volatility_90d_pct: daily_volatility(daily, 90),
    }
}

fn classify_regime(
    cycle: &HalvingCycleSnapshot,
    trend: &MacroTrendSnapshot,
    momentum: &MacroMomentumSnapshot,
) -> MacroRegime {
    let drawdown = trend.drawdown_from_window_ath_pct;
    let price_vs_200w = trend.price_vs_200w_pct.unwrap_or(0.0);
    let change_90d = momentum.change_90d_pct.unwrap_or(0.0);

    if price_vs_200w < -0.10 || drawdown <= -0.55 {
        if change_90d > 0.08 {
            MacroRegime::BearMarketRally
        } else {
            MacroRegime::BearMarket
        }
    } else if drawdown <= -0.35 && change_90d > 0.05 {
        MacroRegime::BearMarketRally
    } else if drawdown <= -0.40 && price_vs_200w.abs() <= 0.20 {
        MacroRegime::BottomingAccumulation
    } else if cycle.days_since_halving >= 540 && drawdown <= -0.20 {
        MacroRegime::LateCycleDistribution
    } else if drawdown > -0.25 && price_vs_200w > 0.15 {
        MacroRegime::BullExpansion
    } else {
        MacroRegime::Neutral
    }
}

fn regime_confidence(trend: &MacroTrendSnapshot, momentum: &MacroMomentumSnapshot) -> u8 {
    let mut confidence = 45;
    if trend.ma_200w.is_some() {
        confidence += 20;
    }
    if momentum.change_90d_pct.is_some() {
        confidence += 15;
    }
    if momentum.change_26w_pct.is_some() {
        confidence += 10;
    }
    confidence.min(90)
}

fn historical_analogs(
    regime: &MacroRegime,
    cycle: &HalvingCycleSnapshot,
    trend: &MacroTrendSnapshot,
    momentum: &MacroMomentumSnapshot,
) -> Vec<HistoricalAnalog> {
    let components = analog_rule_components(cycle, trend, momentum);
    let score = components
        .iter()
        .map(|component| component.points)
        .sum::<u8>();
    let mut rationale = vec![
        format!("cycle_day={}", cycle.days_since_halving),
        format!(
            "drawdown={:.1}%",
            trend.drawdown_from_window_ath_pct * 100.0
        ),
    ];
    if let Some(change_90d) = momentum.change_90d_pct {
        rationale.push(format!("90d_change={:.1}%", change_90d * 100.0));
    }

    let label = match regime {
        MacroRegime::BearMarketRally => "bear_market_slow_rebound",
        MacroRegime::BearMarket => "bear_market_continuation",
        MacroRegime::BottomingAccumulation => "bottoming_accumulation_window",
        MacroRegime::LateCycleDistribution => "late_cycle_distribution",
        MacroRegime::BullExpansion => "bull_expansion",
        MacroRegime::Neutral => "mixed_macro_regime",
    };

    vec![HistoricalAnalog {
        label: label.to_string(),
        score,
        rationale,
        components,
    }]
}

fn analog_rule_components(
    cycle: &HalvingCycleSnapshot,
    trend: &MacroTrendSnapshot,
    momentum: &MacroMomentumSnapshot,
) -> Vec<AnalogScoreComponent> {
    let mut components = Vec::new();
    let cycle_points = if (540..=900).contains(&cycle.days_since_halving) {
        20
    } else if (360..=1_100).contains(&cycle.days_since_halving) {
        12
    } else {
        6
    };
    components.push(AnalogScoreComponent {
        label: "cycle_position".to_string(),
        points: cycle_points,
        max_points: 20,
        detail: format!("cycle_day={}", cycle.days_since_halving),
    });

    let drawdown = trend.drawdown_from_window_ath_pct;
    let drawdown_points = if drawdown <= -0.55 {
        25
    } else if drawdown <= -0.40 {
        20
    } else if drawdown <= -0.25 {
        12
    } else {
        5
    };
    components.push(AnalogScoreComponent {
        label: "drawdown".to_string(),
        points: drawdown_points,
        max_points: 25,
        detail: format!("{:.1}% from window ATH", drawdown * 100.0),
    });

    if let Some(price_vs_200w) = trend.price_vs_200w_pct {
        let ma_points = if price_vs_200w.abs() <= 0.10 {
            20
        } else if price_vs_200w.abs() <= 0.20 {
            14
        } else {
            6
        };
        components.push(AnalogScoreComponent {
            label: "weekly_200ma_proximity".to_string(),
            points: ma_points,
            max_points: 20,
            detail: format!("price_vs_200w={:.1}%", price_vs_200w * 100.0),
        });
    }

    if let Some(change_90d) = momentum.change_90d_pct {
        let momentum_points = if change_90d.abs() <= 0.08 {
            15
        } else if change_90d < 0.0 {
            10
        } else {
            8
        };
        components.push(AnalogScoreComponent {
            label: "quarter_momentum".to_string(),
            points: momentum_points,
            max_points: 15,
            detail: format!("90d_change={:.1}%", change_90d * 100.0),
        });
    }

    components
}

fn historical_analog_comparisons(daily: &[Candle]) -> Vec<AnalogComparisonSet> {
    [30, 90, 180, 365]
        .into_iter()
        .map(|timeframe_days| analog_comparison_for_period(daily, timeframe_days))
        .collect()
}

fn analog_comparison_for_period(daily: &[Candle], timeframe_days: usize) -> AnalogComparisonSet {
    let Some(current) = kline_summary_for_last_window(daily, timeframe_days) else {
        return AnalogComparisonSet {
            timeframe_days,
            current: None,
            matches: Vec::new(),
        };
    };

    let current_start = daily.len().saturating_sub(timeframe_days + 1);
    let latest_allowed_start = current_start.saturating_sub(timeframe_days + 1);
    let stride = (timeframe_days / 6).max(1);
    let mut matches = Vec::new();
    for start in (0..=latest_allowed_start).step_by(stride) {
        let end = start + timeframe_days;
        let forward_end = end + timeframe_days;
        if end >= current_start || forward_end >= daily.len() {
            continue;
        }
        if let Some(candidate) = kline_summary_for_slice(&daily[start..=end]) {
            let forward =
                kline_summary_for_forward_slice(&daily[(end + 1)..=forward_end], daily[end].close);
            matches.push(analog_match_from_candidate(
                timeframe_days,
                &current,
                candidate,
                forward,
            ));
        }
    }
    matches.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.end_ts_ms.cmp(&left.end_ts_ms))
    });
    matches.truncate(3);

    AnalogComparisonSet {
        timeframe_days,
        current: Some(current),
        matches,
    }
}

fn kline_summary_for_last_window(
    daily: &[Candle],
    timeframe_days: usize,
) -> Option<AnalogPathSummary> {
    if daily.len() < timeframe_days + 1 {
        return None;
    }
    kline_summary_for_slice(&daily[(daily.len() - timeframe_days - 1)..])
}

fn kline_summary_for_slice(candles: &[Candle]) -> Option<AnalogPathSummary> {
    let first = candles.first()?;
    kline_summary_for_forward_slice(candles, first.close)
}

fn kline_summary_for_forward_slice(
    candles: &[Candle],
    base_close: f64,
) -> Option<AnalogPathSummary> {
    let first = candles.first()?;
    if base_close <= 0.0 || !base_close.is_finite() {
        return None;
    }
    let mut path = Vec::with_capacity(candles.len());
    let mut normalized_candles = Vec::with_capacity(candles.len());
    for candle in candles {
        if candle.open <= 0.0
            || candle.high <= 0.0
            || candle.low <= 0.0
            || candle.close <= 0.0
            || !candle.open.is_finite()
            || !candle.high.is_finite()
            || !candle.low.is_finite()
            || !candle.close.is_finite()
        {
            return None;
        }
        let offset_days = days_between(first.ts_ms, candle.ts_ms);
        path.push(AnalogPathPoint {
            offset_days,
            return_pct: candle.close / base_close - 1.0,
        });
        normalized_candles.push(AnalogKline {
            ts_ms: candle.ts_ms,
            offset_days,
            open: candle.open,
            high: candle.high,
            low: candle.low,
            close: candle.close,
            index_open: candle.open / base_close * 100.0,
            index_high: candle.high / base_close * 100.0,
            index_low: candle.low / base_close * 100.0,
            index_close: candle.close / base_close * 100.0,
        });
    }
    let final_return_pct = path.last()?.return_pct;
    let max_drawdown_pct = normalized_candles
        .iter()
        .map(|candle| candle.index_low / 100.0 - 1.0)
        .fold(0.0_f64, f64::min);
    let max_runup_pct = normalized_candles
        .iter()
        .map(|candle| candle.index_high / 100.0 - 1.0)
        .fold(0.0_f64, f64::max);

    Some(AnalogPathSummary {
        start_ts_ms: first.ts_ms,
        end_ts_ms: candles.last()?.ts_ms,
        final_return_pct,
        max_drawdown_pct,
        max_runup_pct,
        candles: normalized_candles,
        path,
    })
}

fn analog_match_from_candidate(
    timeframe_days: usize,
    current: &AnalogPathSummary,
    lookback: AnalogPathSummary,
    forward: Option<AnalogPathSummary>,
) -> AnalogMatch {
    let path_rmse = path_rmse(&current.path, &lookback.path);
    let drawdown_diff = (current.max_drawdown_pct - lookback.max_drawdown_pct).abs();
    let runup_diff = (current.max_runup_pct - lookback.max_runup_pct).abs();
    let final_diff = (current.final_return_pct - lookback.final_return_pct).abs();
    let components = vec![
        distance_component("path_shape", 45, path_rmse, 0.35),
        distance_component("drawdown", 20, drawdown_diff, 0.30),
        distance_component("runup", 15, runup_diff, 0.30),
        distance_component("final_return", 20, final_diff, 0.35),
    ];
    let score = components
        .iter()
        .map(|component| component.points)
        .sum::<u8>();
    let display_summary = forward.as_ref().unwrap_or(&lookback);
    let display_start_ts_ms = display_summary.start_ts_ms;
    let display_end_ts_ms = display_summary.end_ts_ms;
    let display_final_return_pct = display_summary.final_return_pct;
    let display_max_drawdown_pct = display_summary.max_drawdown_pct;
    let display_max_runup_pct = display_summary.max_runup_pct;
    let display_path = display_summary.path.clone();

    AnalogMatch {
        id: format!("{}-{}", timeframe_days, lookback.end_ts_ms),
        label: format!(
            "after {}D {}",
            timeframe_days,
            Utc.timestamp_millis_opt(lookback.end_ts_ms)
                .single()
                .map(|dt| dt.format("%Y/%m/%d").to_string())
                .unwrap_or_else(|| lookback.end_ts_ms.to_string())
        ),
        score,
        start_ts_ms: display_start_ts_ms,
        end_ts_ms: display_end_ts_ms,
        final_return_pct: display_final_return_pct,
        max_drawdown_pct: display_max_drawdown_pct,
        max_runup_pct: display_max_runup_pct,
        components,
        lookback,
        forward,
        path: display_path,
    }
}

fn path_rmse(left: &[AnalogPathPoint], right: &[AnalogPathPoint]) -> f64 {
    let len = left.len().min(right.len());
    if len == 0 {
        return 1.0;
    }
    let sum = left
        .iter()
        .zip(right.iter())
        .take(len)
        .map(|(left, right)| (left.return_pct - right.return_pct).powi(2))
        .sum::<f64>();
    (sum / len as f64).sqrt()
}

fn distance_component(
    label: &str,
    max_points: u8,
    distance: f64,
    tolerance: f64,
) -> AnalogScoreComponent {
    let ratio = (1.0 - distance / tolerance).clamp(0.0, 1.0);
    AnalogScoreComponent {
        label: label.to_string(),
        points: (max_points as f64 * ratio).round() as u8,
        max_points,
        detail: format!("distance={:.2}%", distance * 100.0),
    }
}

fn trading_bias(
    regime: &MacroRegime,
    trend: &MacroTrendSnapshot,
    events: &[MacroEvent],
) -> Vec<String> {
    let mut bias = Vec::new();
    match regime {
        MacroRegime::BearMarket | MacroRegime::BearMarketRally => {
            bias.push("alts_rebounds_should_be_treated_as_lower_confidence_longs".to_string());
            bias.push("trend_shorts_near_resistance_keep_priority".to_string());
        }
        MacroRegime::BottomingAccumulation => {
            bias.push("spot_or_low_leverage_core_entries_can_be_scored_higher".to_string());
            bias.push("wait_for_weekly_confirmation_before_aggressive_longs".to_string());
        }
        MacroRegime::BullExpansion => {
            bias.push("trend_longs_can_receive_macro_tailwind".to_string());
        }
        MacroRegime::LateCycleDistribution => {
            bias.push("avoid_chasing_extended_altcoin_breakouts".to_string());
        }
        MacroRegime::Neutral => {
            bias.push("macro_filter_is_mixed_keep_intraday_risk_tight".to_string());
        }
    }

    if trend.price_vs_200w_pct.is_some_and(|value| value < 0.05) {
        bias.push("weekly_200ma_area_requires_position_size_discount".to_string());
    }
    if events
        .iter()
        .any(|event| event.id == "us_midterm_2026" && event.days_to_event.abs() <= 120)
    {
        bias.push("us_midterm_window_policy_uncertainty".to_string());
    }

    bias
}

fn regime_summary(
    regime: &MacroRegime,
    cycle: &HalvingCycleSnapshot,
    trend: &MacroTrendSnapshot,
) -> String {
    format!(
        "{:?}; cycle day {}, quarter {}, drawdown {:.1}%, 200W MA status {}",
        regime,
        cycle.days_since_halving,
        cycle.cycle_quarter,
        trend.drawdown_from_window_ath_pct * 100.0,
        trend.weekly_ma200_status
    )
}

fn us_midterm_2026_event(now_ms: i64) -> MacroEvent {
    let date_ms = utc_ms(2026, 11, 3);
    let days_to_event = days_between(now_ms, date_ms);
    let phase = if days_to_event > 120 {
        "pre_election_background"
    } else if days_to_event >= 0 {
        "pre_election_q4_window"
    } else if days_to_event >= -60 {
        "post_election_resolution_window"
    } else {
        "completed"
    };

    MacroEvent {
        id: "us_midterm_2026".to_string(),
        title: "2026 US midterm elections".to_string(),
        event_type: "us_midterm".to_string(),
        date_ms,
        days_to_event,
        phase: phase.to_string(),
        impact_tags: vec![
            "policy_uncertainty".to_string(),
            "risk_sentiment_shift".to_string(),
            "crypto_regulation_expectation".to_string(),
        ],
    }
}

fn moving_average(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period {
        return None;
    }
    let sum = candles
        .iter()
        .rev()
        .take(period)
        .map(|candle| candle.close)
        .sum::<f64>();
    Some(sum / period as f64)
}

fn change_over_bars(candles: &[Candle], bars: usize) -> Option<f64> {
    if candles.len() <= bars {
        return None;
    }
    let latest = candles.last()?;
    let previous = candles.iter().rev().nth(bars)?;
    if previous.close <= 0.0 {
        return None;
    }
    Some(latest.close / previous.close - 1.0)
}

fn daily_volatility(candles: &[Candle], bars: usize) -> Option<f64> {
    if candles.len() <= bars {
        return None;
    }
    let returns: Vec<_> = candles
        .windows(2)
        .rev()
        .take(bars)
        .filter_map(|window| {
            let previous = window[0].close;
            let latest = window[1].close;
            (previous > 0.0).then_some(latest / previous - 1.0)
        })
        .collect();
    if returns.len() < 2 {
        return None;
    }
    let mean = returns.iter().sum::<f64>() / returns.len() as f64;
    let variance = returns
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (returns.len() - 1) as f64;
    Some(variance.sqrt() * (365.0_f64).sqrt())
}

fn weekly_from_daily(daily: &[Candle]) -> Vec<Candle> {
    daily
        .chunks(7)
        .filter(|chunk| chunk.len() == 7)
        .map(|chunk| {
            let first = &chunk[0];
            let last = &chunk[chunk.len() - 1];
            Candle {
                ts_ms: first.ts_ms,
                open: first.open,
                high: chunk
                    .iter()
                    .map(|candle| candle.high)
                    .fold(f64::MIN, f64::max),
                low: chunk
                    .iter()
                    .map(|candle| candle.low)
                    .fold(f64::MAX, f64::min),
                close: last.close,
                volume: chunk.iter().map(|candle| candle.volume).sum(),
            }
        })
        .collect()
}

fn utc_ms(year: i32, month: u32, day: u32) -> i64 {
    Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .expect("static macro dates should be valid")
        .timestamp_millis()
}

fn days_between(start_ms: i64, end_ms: i64) -> i64 {
    (end_ms - start_ms) / 86_400_000
}

#[allow(dead_code)]
fn calendar_quarter(ts_ms: i64) -> u8 {
    let dt = Utc
        .timestamp_millis_opt(ts_ms)
        .single()
        .unwrap_or_else(Utc::now);
    ((dt.month() - 1) / 3 + 1) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weekly_candles(latest_close: f64) -> Vec<Candle> {
        (0..220)
            .map(|index| {
                let close = if index == 219 {
                    latest_close
                } else {
                    45_000.0 + index as f64 * 80.0
                };
                Candle {
                    ts_ms: index as i64 * 604_800_000,
                    open: close,
                    high: close * 1.02,
                    low: close * 0.98,
                    close,
                    volume: 100.0,
                }
            })
            .collect()
    }

    fn daily_candles(latest_close: f64) -> Vec<Candle> {
        (0..120)
            .map(|index| {
                let close = if index == 119 {
                    latest_close
                } else {
                    55_000.0 + index as f64 * 20.0
                };
                Candle {
                    ts_ms: index as i64 * 86_400_000,
                    open: close,
                    high: close * 1.01,
                    low: close * 0.99,
                    close,
                    volume: 100.0,
                }
            })
            .collect()
    }

    fn long_daily_candles(latest_close: f64) -> Vec<Candle> {
        let start_ms = utc_ms(2022, 1, 1);
        (0..900)
            .map(|index| {
                let wave = ((index % 120) as f64 - 60.0) * 120.0;
                let drift = index as f64 * 18.0;
                let close = if index == 899 {
                    latest_close
                } else {
                    42_000.0 + drift + wave
                };
                Candle {
                    ts_ms: start_ms + index as i64 * 86_400_000,
                    open: close,
                    high: close * 1.01,
                    low: close * 0.99,
                    close,
                    volume: 100.0,
                }
            })
            .collect()
    }

    #[test]
    fn builds_macro_snapshot_with_halving_and_midterm_context() {
        let now_ms = utc_ms(2026, 6, 29);
        let snapshot =
            build_btc_macro_snapshot(&weekly_candles(60_000.0), &daily_candles(60_000.0), now_ms)
                .unwrap();

        assert_eq!(snapshot.asset, "BTC");
        assert_eq!(snapshot.cycle.cycle_year, 3);
        assert!(snapshot.cycle.days_since_halving > 700);
        assert_eq!(snapshot.events[0].id, "us_midterm_2026");
        assert!(snapshot.events[0].days_to_event > 0);
        assert!(snapshot.trend.ma_200w.is_some());
        assert_eq!(snapshot.valuation_metrics.len(), 2);
    }

    #[test]
    fn classifies_bear_market_rally_when_deep_drawdown_rebounds() {
        let mut weekly = weekly_candles(50_000.0);
        weekly[180].high = 120_000.0;
        let mut daily = daily_candles(50_000.0);
        daily[29].close = 40_000.0;
        daily[119].close = 50_000.0;

        let snapshot = build_btc_macro_snapshot(&weekly, &daily, utc_ms(2026, 6, 29)).unwrap();

        assert_eq!(snapshot.regime, MacroRegime::BearMarketRally);
        assert!(
            snapshot
                .trading_bias
                .contains(&"alts_rebounds_should_be_treated_as_lower_confidence_longs".to_string())
        );
    }

    #[test]
    fn macro_snapshot_includes_ahr999_history_and_analog_comparisons() {
        let snapshot = build_btc_macro_snapshot(
            &weekly_candles(60_000.0),
            &long_daily_candles(60_000.0),
            utc_ms(2026, 6, 29),
        )
        .unwrap();

        let ahr999 = snapshot.ahr999_history.as_ref().unwrap();
        assert!(ahr999.points.len() > 600);
        assert_eq!(ahr999.bands.len(), 4);
        assert!(
            snapshot
                .analog_comparisons
                .iter()
                .any(|comparison| comparison.timeframe_days == 90 && !comparison.matches.is_empty())
        );
        let comparison = snapshot
            .analog_comparisons
            .iter()
            .find(|comparison| comparison.timeframe_days == 90)
            .unwrap();
        let match_window = comparison.matches.first().unwrap();
        assert_eq!(comparison.current.as_ref().unwrap().candles.len(), 91);
        assert_eq!(match_window.lookback.candles.len(), 91);
        assert_eq!(match_window.forward.as_ref().unwrap().candles.len(), 90);
        assert!(
            match_window.forward.as_ref().unwrap().start_ts_ms > match_window.lookback.end_ts_ms
        );
        assert!(
            snapshot.analogs[0]
                .components
                .iter()
                .any(|component| component.label == "drawdown")
        );
    }

    #[test]
    fn aggregates_daily_candles_into_weekly_fallback() {
        let daily = long_daily_candles(60_000.0);

        let weekly = weekly_from_daily(&daily[..14]);

        assert_eq!(weekly.len(), 2);
        assert_eq!(weekly[0].ts_ms, daily[0].ts_ms);
        assert_eq!(weekly[0].open, daily[0].open);
        assert_eq!(weekly[0].close, daily[6].close);
        assert_eq!(
            weekly[0].high,
            daily[..7]
                .iter()
                .map(|candle| candle.high)
                .fold(f64::MIN, f64::max)
        );
        assert_eq!(
            weekly[0].volume,
            daily[..7].iter().map(|candle| candle.volume).sum::<f64>()
        );
    }
}
