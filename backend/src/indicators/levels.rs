use crate::domain::{Candle, LevelKind, LevelZone};

#[derive(Debug, Clone, Copy)]
pub struct LevelConfig {
    pub cluster_pct: f64,
    pub min_touches: usize,
}

pub fn find_levels(candles: &[Candle], current_price: f64, config: LevelConfig) -> Vec<LevelZone> {
    if candles.is_empty() || current_price <= 0.0 {
        return Vec::new();
    }

    let mut levels = Vec::new();
    levels.extend(cluster_prices(
        candles.iter().map(|candle| candle.low).collect(),
        LevelKind::Support,
        current_price,
        config,
    ));
    levels.extend(cluster_prices(
        candles.iter().map(|candle| candle.high).collect(),
        LevelKind::Resistance,
        current_price,
        config,
    ));
    levels.sort_by(|left, right| {
        left.distance_pct
            .partial_cmp(&right.distance_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    levels
}

fn cluster_prices(
    mut prices: Vec<f64>,
    kind: LevelKind,
    current_price: f64,
    config: LevelConfig,
) -> Vec<LevelZone> {
    prices.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));

    let mut zones = Vec::new();
    let mut index = 0;
    while index < prices.len() {
        let anchor = prices[index];
        let mut cluster = vec![anchor];
        index += 1;

        while index < prices.len()
            && anchor > 0.0
            && (prices[index] - anchor).abs() / anchor <= config.cluster_pct
        {
            cluster.push(prices[index]);
            index += 1;
        }

        if cluster.len() >= config.min_touches {
            let lower = cluster.iter().copied().fold(f64::INFINITY, f64::min);
            let upper = cluster.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            zones.push(LevelZone {
                kind,
                lower,
                upper,
                touches: cluster.len(),
                distance_pct: distance_pct(current_price, lower, upper),
            });
        }
    }

    zones
}

fn distance_pct(price: f64, lower: f64, upper: f64) -> f64 {
    if price >= lower && price <= upper {
        0.0
    } else if price < lower {
        (lower - price) / price
    } else {
        (price - upper) / price
    }
}
