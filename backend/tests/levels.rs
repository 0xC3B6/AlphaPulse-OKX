use alphapulse_okx_backend::{
    domain::{Candle, LevelKind},
    indicators::levels::{find_levels, LevelConfig},
};

fn candle(ts_ms: i64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle {
        ts_ms,
        open: close,
        high,
        low,
        close,
        volume,
    }
}

#[test]
fn clusters_repeated_support_levels() {
    let candles = vec![
        candle(1, 18.0, 15.2, 16.0, 100.0),
        candle(2, 17.0, 15.3, 16.2, 130.0),
        candle(3, 18.2, 15.25, 17.5, 160.0),
        candle(4, 19.0, 16.4, 18.5, 100.0),
    ];

    let levels = find_levels(
        &candles,
        16.0,
        LevelConfig {
            cluster_pct: 0.01,
            min_touches: 2,
        },
    );

    let support = levels
        .iter()
        .find(|level| level.kind == LevelKind::Support)
        .unwrap();
    assert!(support.lower <= 15.2);
    assert!(support.upper >= 15.3);
    assert_eq!(support.touches, 3);
}

#[test]
fn clusters_repeated_resistance_levels() {
    let candles = vec![
        candle(1, 20.0, 17.0, 18.0, 100.0),
        candle(2, 20.2, 18.0, 19.0, 120.0),
        candle(3, 20.1, 18.4, 19.5, 110.0),
        candle(4, 18.8, 16.4, 17.0, 180.0),
    ];

    let levels = find_levels(
        &candles,
        19.0,
        LevelConfig {
            cluster_pct: 0.015,
            min_touches: 2,
        },
    );

    let resistance = levels
        .iter()
        .find(|level| level.kind == LevelKind::Resistance)
        .unwrap();
    assert!(resistance.lower <= 20.0);
    assert!(resistance.upper >= 20.2);
    assert_eq!(resistance.touches, 3);
}
