use alphapulse_okx_backend::{
    domain::Candle,
    quality::{classify_history, UniversePolicy},
};

const HOUR_MS: i64 = 3_600_000;

#[test]
fn rejects_symbols_with_less_than_three_days_of_history() {
    let candles = hourly_candles(48);

    let decision = classify_history(&candles, UniversePolicy::default());

    assert!(!decision.allowed);
    assert!(decision.tags.is_empty());
}

#[test]
fn tags_symbols_with_less_than_seven_days_of_history() {
    let candles = hourly_candles(96);

    let decision = classify_history(&candles, UniversePolicy::default());

    assert!(decision.allowed);
    assert!(decision.tags.contains(&"thin_history".to_string()));
}

#[test]
fn accepts_symbols_with_at_least_seven_days_of_history_without_tag() {
    let candles = hourly_candles(190);

    let decision = classify_history(&candles, UniversePolicy::default());

    assert!(decision.allowed);
    assert!(decision.tags.is_empty());
}

fn hourly_candles(count: usize) -> Vec<Candle> {
    (0..count)
        .map(|index| Candle {
            ts_ms: index as i64 * HOUR_MS,
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.0,
            volume: 100.0,
        })
        .collect()
}
