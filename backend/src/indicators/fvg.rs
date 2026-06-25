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
                let filled = later.iter().any(|candle| candle.low <= upper);
                zones.push(FvgZone {
                    timeframe,
                    direction: Direction::Long,
                    lower,
                    upper,
                    gap_pct,
                    distance_pct: zone_distance_pct(current_price, lower, upper),
                    filled,
                });
            }
        }

        if first.low > third.high {
            let lower = third.high;
            let upper = first.low;
            let gap_pct = (upper - lower) / lower;
            if gap_pct >= min_gap_pct {
                let filled = later.iter().any(|candle| candle.high >= lower);
                zones.push(FvgZone {
                    timeframe,
                    direction: Direction::Short,
                    lower,
                    upper,
                    gap_pct,
                    distance_pct: zone_distance_pct(current_price, lower, upper),
                    filled,
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
