use crate::domain::{Candle, Direction, FvgZone, Timeframe};

pub fn detect_fvgs(
    candles: &[Candle],
    timeframe: Timeframe,
    min_gap_pct: f64,
    current_price: f64,
) -> Vec<FvgZone> {
    if candles.len() < 3 || current_price <= 0.0 {
        return Vec::new();
    }

    let mut zones = Vec::new();

    for index in 0..=(candles.len() - 3) {
        let first = &candles[index];
        let third = &candles[index + 2];
        let later = &candles[(index + 3)..];

        if first.high < third.low {
            let lower = first.high;
            let upper = third.low;
            let gap_pct = (upper - lower) / lower;
            if gap_pct >= min_gap_pct {
                let filled_at = later.iter().find(|candle| candle.low <= lower);
                zones.push(FvgZone {
                    timeframe,
                    direction: Direction::Long,
                    start_ts_ms: first.ts_ms,
                    end_ts_ms: filled_at.map(|candle| candle.ts_ms).unwrap_or_else(|| {
                        candles
                            .last()
                            .map(|candle| candle.ts_ms)
                            .unwrap_or(first.ts_ms)
                    }),
                    lower,
                    upper,
                    gap_pct,
                    distance_pct: zone_distance_pct(current_price, lower, upper),
                    filled: filled_at.is_some(),
                });
            }
        }

        if first.low > third.high {
            let lower = third.high;
            let upper = first.low;
            let gap_pct = (upper - lower) / lower;
            if gap_pct >= min_gap_pct {
                let filled_at = later.iter().find(|candle| candle.high >= upper);
                zones.push(FvgZone {
                    timeframe,
                    direction: Direction::Short,
                    start_ts_ms: first.ts_ms,
                    end_ts_ms: filled_at.map(|candle| candle.ts_ms).unwrap_or_else(|| {
                        candles
                            .last()
                            .map(|candle| candle.ts_ms)
                            .unwrap_or(first.ts_ms)
                    }),
                    lower,
                    upper,
                    gap_pct,
                    distance_pct: zone_distance_pct(current_price, lower, upper),
                    filled: filled_at.is_some(),
                });
            }
        }
    }

    zones
}

fn zone_distance_pct(price: f64, lower: f64, upper: f64) -> f64 {
    if price >= lower && price <= upper {
        0.0
    } else if price < lower {
        (lower - price) / price
    } else {
        (price - upper) / price
    }
}
