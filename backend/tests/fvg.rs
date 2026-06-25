use alphapulse_okx_backend::{
    domain::{Candle, Direction, Timeframe},
    indicators::fvg::detect_fvgs,
};

fn candle(ts_ms: i64, high: f64, low: f64, close: f64) -> Candle {
    Candle {
        ts_ms,
        open: close,
        high,
        low,
        close,
        volume: 100.0,
    }
}

#[test]
fn detects_bullish_three_candle_gap() {
    let candles = vec![
        candle(1, 10.0, 9.5, 9.8),
        candle(2, 10.4, 9.7, 10.1),
        candle(3, 11.4, 10.8, 11.2),
    ];

    let zones = detect_fvgs(&candles, Timeframe::M15, 0.02, 11.2);

    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0].direction, Direction::Long);
    assert_eq!(zones[0].lower, 10.0);
    assert_eq!(zones[0].upper, 10.8);
    assert!(!zones[0].filled);
}

#[test]
fn marks_bullish_gap_filled_when_later_low_revisits_zone() {
    let candles = vec![
        candle(1, 10.0, 9.5, 9.8),
        candle(2, 10.4, 9.7, 10.1),
        candle(3, 11.4, 10.8, 11.2),
        candle(4, 11.1, 10.6, 10.7),
    ];

    let zones = detect_fvgs(&candles, Timeframe::M15, 0.02, 10.7);

    assert_eq!(zones.len(), 1);
    assert!(zones[0].filled);
}

#[test]
fn detects_bearish_three_candle_gap() {
    let candles = vec![
        candle(1, 20.4, 20.0, 20.2),
        candle(2, 20.2, 19.4, 19.8),
        candle(3, 19.1, 18.5, 18.8),
    ];

    let zones = detect_fvgs(&candles, Timeframe::M5, 0.02, 18.8);

    assert_eq!(zones.len(), 1);
    assert_eq!(zones[0].direction, Direction::Short);
    assert_eq!(zones[0].lower, 19.1);
    assert_eq!(zones[0].upper, 20.0);
}
