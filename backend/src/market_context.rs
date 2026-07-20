use crate::domain::Candle;

pub const INTRADAY_UP_EXTREME: &str = "intraday_up_extreme";
pub const INTRADAY_DOWN_EXTREME: &str = "intraday_down_extreme";
pub const MULTIDAY_UP_EXTREME: &str = "multiday_up_extreme";
pub const MULTIDAY_DOWN_EXTREME: &str = "multiday_down_extreme";

const INTRADAY_WINDOW_HOURS: usize = 6;
const INTRADAY_MOVE_THRESHOLD: f64 = 0.08;
const DAILY_TICKER_MOVE_THRESHOLD: f64 = 0.12;
const MULTIDAY_WINDOW_HOURS: usize = 72;
const MULTIDAY_MOVE_THRESHOLD: f64 = 0.20;
const CONSECUTIVE_DAILY_MIN_MOVE: f64 = 0.025;
const DISTRIBUTED_MULTIDAY_MIN_DAYS: usize = 2;

pub fn classify_overextension(candles_1h: &[Candle], change_24h_pct: f64) -> Vec<&'static str> {
    let intraday_change =
        window_change(candles_1h, INTRADAY_WINDOW_HOURS).unwrap_or(change_24h_pct);
    let consecutive_up = consecutive_daily_direction(candles_1h, DirectionHint::Up);
    let consecutive_down = consecutive_daily_direction(candles_1h, DirectionHint::Down);
    let distributed_up = distributed_multiday_direction(candles_1h, DirectionHint::Up);
    let distributed_down = distributed_multiday_direction(candles_1h, DirectionHint::Down);
    let mut tags = Vec::new();

    if intraday_change >= INTRADAY_MOVE_THRESHOLD || change_24h_pct >= DAILY_TICKER_MOVE_THRESHOLD {
        tags.push(INTRADAY_UP_EXTREME);
    }
    if intraday_change <= -INTRADAY_MOVE_THRESHOLD || change_24h_pct <= -DAILY_TICKER_MOVE_THRESHOLD
    {
        tags.push(INTRADAY_DOWN_EXTREME);
    }
    if consecutive_up || distributed_up {
        tags.push(MULTIDAY_UP_EXTREME);
    }
    if consecutive_down || distributed_down {
        tags.push(MULTIDAY_DOWN_EXTREME);
    }

    tags
}

fn window_change(candles: &[Candle], hours: usize) -> Option<f64> {
    if candles.len() < hours.max(2) {
        return None;
    }
    let latest = candles.last()?;
    let first = candles.get(candles.len().saturating_sub(hours))?;
    if first.open <= 0.0 {
        return None;
    }
    Some(latest.close / first.open - 1.0)
}

fn distributed_multiday_direction(candles: &[Candle], direction: DirectionHint) -> bool {
    let total_change = window_change(candles, MULTIDAY_WINDOW_HOURS).unwrap_or(0.0);
    let total_matches = match direction {
        DirectionHint::Up => total_change >= MULTIDAY_MOVE_THRESHOLD,
        DirectionHint::Down => total_change <= -MULTIDAY_MOVE_THRESHOLD,
    };
    if !total_matches {
        return false;
    }

    daily_bucket_changes(candles, MULTIDAY_WINDOW_HOURS)
        .into_iter()
        .filter(|change| match direction {
            DirectionHint::Up => *change >= CONSECUTIVE_DAILY_MIN_MOVE,
            DirectionHint::Down => *change <= -CONSECUTIVE_DAILY_MIN_MOVE,
        })
        .count()
        >= DISTRIBUTED_MULTIDAY_MIN_DAYS
}

fn daily_bucket_changes(candles: &[Candle], hours: usize) -> Vec<f64> {
    if candles.len() < hours {
        return Vec::new();
    }

    let start_index = candles.len().saturating_sub(hours);
    candles[start_index..]
        .chunks(24)
        .filter_map(|bucket| {
            let first = bucket.first()?;
            let last = bucket.last()?;
            if first.open <= 0.0 {
                return None;
            }
            Some(last.close / first.open - 1.0)
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
enum DirectionHint {
    Up,
    Down,
}

fn consecutive_daily_direction(candles: &[Candle], direction: DirectionHint) -> bool {
    if candles.len() < 72 {
        return false;
    }

    (0..3).all(|day_offset| {
        let end = candles.len().saturating_sub(day_offset * 24);
        let start = end.saturating_sub(24);
        let Some(first) = candles.get(start) else {
            return false;
        };
        let Some(last) = candles.get(end.saturating_sub(1)) else {
            return false;
        };
        if first.open <= 0.0 {
            return false;
        }
        let change = last.close / first.open - 1.0;
        match direction {
            DirectionHint::Up => change >= CONSECUTIVE_DAILY_MIN_MOVE,
            DirectionHint::Down => change <= -CONSECUTIVE_DAILY_MIN_MOVE,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candles_from_closes(closes: &[f64]) -> Vec<Candle> {
        closes
            .iter()
            .enumerate()
            .map(|(index, close)| Candle {
                ts_ms: index as i64 * 3_600_000,
                open: if index == 0 {
                    *close
                } else {
                    closes[index - 1]
                },
                high: close.max(if index == 0 {
                    *close
                } else {
                    closes[index - 1]
                }),
                low: close.min(if index == 0 {
                    *close
                } else {
                    closes[index - 1]
                }),
                close: *close,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn tags_intraday_up_extreme_without_predicting_reversal() {
        let candles = candles_from_closes(&[100.0, 101.0, 102.0, 104.0, 106.0, 109.0]);

        let tags = classify_overextension(&candles, 0.09);

        assert!(tags.contains(&INTRADAY_UP_EXTREME));
        assert!(!tags.contains(&INTRADAY_DOWN_EXTREME));
    }

    #[test]
    fn tags_multiday_up_extreme_from_three_daily_pushes() {
        let closes: Vec<f64> = (0..72)
            .map(|hour| {
                let day = hour / 24;
                let intra = (hour % 24) as f64 / 24.0;
                100.0 * 1.03_f64.powi(day) * (1.0 + intra * 0.03)
            })
            .collect();
        let candles = candles_from_closes(&closes);

        let tags = classify_overextension(&candles, 0.03);

        assert!(tags.contains(&MULTIDAY_UP_EXTREME));
    }

    #[test]
    fn does_not_tag_single_day_pump_as_multiday_up_extreme() {
        let closes: Vec<f64> = (0..72)
            .map(|hour| {
                if hour < 48 {
                    100.0 - (hour as f64 * 0.03)
                } else {
                    98.5 + ((hour - 48) as f64 * 1.35)
                }
            })
            .collect();
        let candles = candles_from_closes(&closes);

        let tags = classify_overextension(&candles, 0.35);

        assert!(tags.contains(&INTRADAY_UP_EXTREME));
        assert!(!tags.contains(&MULTIDAY_UP_EXTREME));
    }
}
