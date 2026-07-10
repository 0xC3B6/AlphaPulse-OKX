use alphapulse_okx_backend::{
    config::AppConfig,
    domain::{
        Direction, PatternKind, PatternLevelZone, PatternSignal, PatternStatus, ScalpingMetrics,
        Score, Timeframe,
    },
};

#[test]
fn default_config_matches_v1_decisions() {
    let config = AppConfig::default();

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8787);
    assert_eq!(config.scan_interval_secs, 30);
    assert_eq!(config.dynamic_pool_size, 40);
    assert_eq!(config.trend_alert_threshold, 80);
    assert_eq!(config.range_alert_threshold, 80);
    assert_eq!(config.min_listing_age_days, 3.0);
    assert_eq!(config.new_listing_days, 14.0);
    assert_eq!(config.min_history_days, 3.0);
    assert_eq!(config.thin_history_days, 7.0);
    assert!(config
        .fixed_watchlist
        .contains(&"BTC-USDT-SWAP".to_string()));
    assert!(config
        .fixed_watchlist
        .contains(&"LAB-USDT-SWAP".to_string()));
}

#[test]
fn domain_types_serialize_with_stable_names() {
    let score = Score {
        value: 84,
        direction: Direction::Short,
        reasons: vec!["15m drop expanded".to_string(), "volume 3.1x".to_string()],
    };

    let json = serde_json::to_string(&score).unwrap();

    assert!(json.contains("\"value\":84"));
    assert!(json.contains("\"direction\":\"short\""));
    assert!(json.contains("15m drop expanded"));
}

#[test]
fn pattern_signal_serializes_with_stable_names() {
    let signal = PatternSignal {
        kind: PatternKind::DoubleBottom,
        direction: Direction::Long,
        timeframe: Timeframe::M15,
        status: PatternStatus::Holding,
        score: 72,
        structure_score: 42,
        confirmation_score: 14,
        hold_score: 16,
        trade_score: 72,
        neckline: Some(104.0),
        invalidation_level: Some(98.0),
        start_ts_ms: 1,
        confirm_ts_ms: Some(2),
        pivots: vec![],
        level_zone: Some(PatternLevelZone {
            lower: 103.5,
            upper: 104.5,
        }),
        reasons: vec!["neckline retest holding".to_string()],
        warnings: vec!["btc context neutral".to_string()],
    };

    let json = serde_json::to_string(&signal).unwrap();

    assert!(json.contains("\"kind\":\"double_bottom\""));
    assert!(json.contains("\"direction\":\"long\""));
    assert!(json.contains("\"timeframe\":\"m15\""));
    assert!(json.contains("\"status\":\"holding\""));
    assert!(json.contains("\"structure_score\":42"));
    assert!(json.contains("\"trade_score\":72"));
    assert!(json.contains("neckline retest holding"));
    assert!(json.contains("btc context neutral"));
}

#[test]
fn scalping_metrics_serialize_with_stable_names() {
    let metrics = ScalpingMetrics {
        volume_ratio: 2.5,
        vwap: Some(101.25),
        vwap_distance_atr: Some(-0.75),
        latest_move_atr: Some(1.2),
        atr_15m_pct: Some(0.018),
        adx_15m: Some(27.4),
        bollinger_width_pct: Some(0.044),
    };

    let json = serde_json::to_value(&metrics).unwrap();

    assert_eq!(
        json,
        serde_json::json!({
            "volume_ratio": 2.5,
            "vwap": 101.25,
            "vwap_distance_atr": -0.75,
            "latest_move_atr": 1.2,
            "atr_15m_pct": 0.018,
            "adx_15m": 27.4,
            "bollinger_width_pct": 0.044,
        })
    );
}

#[test]
fn timeframe_maps_to_okx_bar_names() {
    assert_eq!(Timeframe::M5.okx_bar(), "5m");
    assert_eq!(Timeframe::M15.okx_bar(), "15m");
    assert_eq!(Timeframe::H1.okx_bar(), "1H");
    assert_eq!(Timeframe::D1.okx_bar(), "1D");
    assert_eq!(Timeframe::W1.okx_bar(), "1W");
}
