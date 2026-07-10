use crate::domain::{Candle, ScalpingMetrics};

const ATR_PERIOD: usize = 14;
const BB_PERIOD: usize = 20;

pub fn scalping_metrics(
    candles_5m: &[Candle],
    candles_15m: &[Candle],
    price: f64,
    volume_ratio: f64,
) -> ScalpingMetrics {
    let atr_abs = atr(candles_15m, ATR_PERIOD);
    let atr_15m_pct = atr_abs.and_then(|value| {
        if price > 0.0 {
            Some(value / price)
        } else {
            None
        }
    });
    let vwap = vwap(candles_5m);
    let vwap_distance_atr = vwap
        .zip(atr_abs)
        .and_then(|(vwap, atr)| (atr > 0.0).then_some((price - vwap) / atr));
    let latest_move_atr = latest_move_atr(candles_15m, atr_abs);

    ScalpingMetrics {
        volume_ratio,
        vwap,
        vwap_distance_atr,
        latest_move_atr,
        atr_15m_pct,
        adx_15m: adx(candles_15m, ATR_PERIOD),
        bollinger_width_pct: bollinger_width_pct(candles_15m, BB_PERIOD),
    }
}

fn vwap(candles: &[Candle]) -> Option<f64> {
    let recent = recent(candles, 96);
    let total_volume = recent.iter().map(|candle| candle.volume).sum::<f64>();
    if total_volume <= 0.0 {
        return None;
    }
    let weighted_price = recent
        .iter()
        .map(|candle| typical_price(candle) * candle.volume)
        .sum::<f64>();
    Some(weighted_price / total_volume)
}

fn atr(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period + 1 {
        return None;
    }
    let window_start = candles.len().saturating_sub(period + 1);
    let mut ranges = Vec::with_capacity(period);
    for pair in candles[window_start..].windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        let true_range = (current.high - current.low)
            .max((current.high - previous.close).abs())
            .max((current.low - previous.close).abs());
        ranges.push(true_range);
    }
    average(&ranges)
}

fn latest_move_atr(candles: &[Candle], atr_abs: Option<f64>) -> Option<f64> {
    let atr = atr_abs?;
    if atr <= 0.0 {
        return None;
    }
    let previous = candles.iter().rev().nth(1)?;
    let latest = candles.last()?;
    Some((latest.close - previous.close) / atr)
}

fn adx(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period * 2 + 1 {
        return None;
    }

    let window_start = candles.len().saturating_sub(period * 2 + 1);
    let candles = &candles[window_start..];
    let mut plus_dm = Vec::new();
    let mut minus_dm = Vec::new();
    let mut true_ranges = Vec::new();

    for pair in candles.windows(2) {
        let previous = &pair[0];
        let current = &pair[1];
        let up_move = current.high - previous.high;
        let down_move = previous.low - current.low;
        plus_dm.push(if up_move > down_move && up_move > 0.0 {
            up_move
        } else {
            0.0
        });
        minus_dm.push(if down_move > up_move && down_move > 0.0 {
            down_move
        } else {
            0.0
        });
        true_ranges.push(
            (current.high - current.low)
                .max((current.high - previous.close).abs())
                .max((current.low - previous.close).abs()),
        );
    }

    let mut dx_values = Vec::new();
    for index in period..=true_ranges.len() {
        let tr_sum = true_ranges[index - period..index].iter().sum::<f64>();
        if tr_sum <= 0.0 {
            continue;
        }
        let plus_di = plus_dm[index - period..index].iter().sum::<f64>() / tr_sum * 100.0;
        let minus_di = minus_dm[index - period..index].iter().sum::<f64>() / tr_sum * 100.0;
        let denominator = plus_di + minus_di;
        if denominator > 0.0 {
            dx_values.push((plus_di - minus_di).abs() / denominator * 100.0);
        }
    }

    average(&dx_values)
}

fn bollinger_width_pct(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < period {
        return None;
    }
    let closes: Vec<_> = candles[candles.len() - period..]
        .iter()
        .map(|candle| candle.close)
        .collect();
    let mean = average(&closes)?;
    if mean <= 0.0 {
        return None;
    }
    let variance = closes
        .iter()
        .map(|close| (close - mean).powi(2))
        .sum::<f64>()
        / closes.len() as f64;
    Some(4.0 * variance.sqrt() / mean)
}

fn typical_price(candle: &Candle) -> f64 {
    (candle.high + candle.low + candle.close) / 3.0
}

fn recent(candles: &[Candle], limit: usize) -> &[Candle] {
    &candles[candles.len().saturating_sub(limit)..]
}

fn average(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then_some(values.iter().sum::<f64>() / values.len() as f64)
}
