use alphapulse_okx_backend::{
    config::AppConfig,
    domain::Candle,
    valuation::{
        apply_ahr999_fallback, parse_coinglass_ahr999, self_calculated_ahr999,
        self_calculated_ahr999_history, valuation_metrics_unavailable,
    },
};

#[test]
fn config_can_read_coinglass_key_from_env_pairs() {
    let config = AppConfig::from_env_pairs([("COINGLASS_API_KEY", "test-key")]);

    assert_eq!(config.coinglass_api_key.as_deref(), Some("test-key"));
}

#[test]
fn parses_latest_coinglass_ahr999_point() {
    let json = r#"{
        "code": "0",
        "msg": "success",
        "data": [
            {
                "date_string": "2026/06/28",
                "average_price": 60111.1,
                "ahr999_value": 0.62,
                "current_value": 60200.0
            },
            {
                "date_string": "2026/06/29",
                "average_price": 60222.2,
                "ahr999_value": 0.64,
                "current_value": 60300.0
            }
        ]
    }"#;

    let metric = parse_coinglass_ahr999(json).unwrap();

    assert_eq!(metric.id, "ahr999");
    assert_eq!(metric.status, "available");
    assert_eq!(metric.value, Some(0.64));
    assert_eq!(metric.date.as_deref(), Some("2026/06/29"));
    assert_eq!(metric.source.as_deref(), Some("coinglass"));
    assert_eq!(metric.note, "current=60300.00 average=60222.20");
}

#[test]
fn unavailable_metrics_keep_macro_snapshot_degraded_not_failed() {
    let metrics = valuation_metrics_unavailable("coinglass api key missing");

    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].id, "ahr999");
    assert_eq!(metrics[0].status, "unavailable");
    assert_eq!(metrics[0].note, "coinglass api key missing");
    assert_eq!(metrics[1].id, "mvrv_z");
    assert_eq!(metrics[1].status, "data_source_pending");
}

#[test]
fn self_calculates_ahr999_from_okx_daily_candles() {
    let candles = sample_daily_candles();

    let metric = self_calculated_ahr999(&candles).unwrap();

    assert_eq!(metric.id, "ahr999");
    assert_eq!(metric.status, "available");
    assert_eq!(metric.source.as_deref(), Some("self_calculated_okx"));
    assert!(metric.value.unwrap() > 0.0);
    assert!(metric.note.contains("gma200="));
    assert!(metric.date.is_some());
    assert!(metric.zone.is_some());
}

#[test]
fn self_calculated_ahr999_replaces_unavailable_coinglass_metric() {
    let mut metrics = valuation_metrics_unavailable("coinglass returned 401: Upgrade plan");
    let fallback = self_calculated_ahr999(&sample_daily_candles()).unwrap();

    apply_ahr999_fallback(&mut metrics, fallback);

    assert_eq!(metrics[0].id, "ahr999");
    assert_eq!(metrics[0].status, "available");
    assert_eq!(metrics[0].source.as_deref(), Some("self_calculated_okx"));
    assert_eq!(metrics[1].id, "mvrv_z");
}

#[test]
fn self_calculated_ahr999_uses_geometric_200_day_average() {
    let candles: Vec<_> = (0..200)
        .map(|index| Candle {
            ts_ms: 1_780_000_000_000 + index * 86_400_000,
            open: 100.0,
            high: 400.0,
            low: 100.0,
            close: if index % 2 == 0 { 100.0 } else { 400.0 },
            volume: 100.0,
        })
        .collect();

    let metric = self_calculated_ahr999(&candles).unwrap();

    assert!(metric.note.contains("gma200=200.00"));
}

#[test]
fn builds_ahr999_history_with_range_counts_and_recommendations() {
    let history = self_calculated_ahr999_history(&sample_daily_candles()).unwrap();

    assert_eq!(history.source, "self_calculated_okx");
    assert_eq!(history.points.len(), 21);
    assert!(history.points[0].value > 0.0);
    assert!(history.points[0].gma200 > 0.0);
    assert!(history.points[0].model_price > 0.0);
    assert_eq!(history.bands.len(), 4);
    assert!(history.bands.iter().any(|band| {
        band.id == "deep_value" && band.upper == Some(0.45) && band.recommendation.contains("spot")
    }));
    assert_eq!(
        history.bands.iter().map(|band| band.days).sum::<usize>(),
        history.points.len()
    );
}

fn sample_daily_candles() -> Vec<Candle> {
    (0..220)
        .map(|index| Candle {
            ts_ms: 1_780_000_000_000 + index * 86_400_000,
            open: 58_000.0,
            high: 61_000.0,
            low: 57_000.0,
            close: 60_000.0 + index as f64,
            volume: 100.0,
        })
        .collect()
}
