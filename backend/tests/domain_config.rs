use alphapulse_okx_backend::{
    config::AppConfig,
    domain::{Direction, Score, Timeframe},
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
fn config_can_override_local_bind_host_and_port_from_env_pairs() {
    let config = AppConfig::from_env_pairs([
        ("ALPHAPULSE_HOST", "127.0.0.1"),
        ("ALPHAPULSE_PORT", "8788"),
    ]);

    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8788);
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
fn timeframe_maps_to_okx_bar_names() {
    assert_eq!(Timeframe::M5.okx_bar(), "5m");
    assert_eq!(Timeframe::M15.okx_bar(), "15m");
    assert_eq!(Timeframe::H1.okx_bar(), "1H");
    assert_eq!(Timeframe::D1.okx_bar(), "1D");
    assert_eq!(Timeframe::W1.okx_bar(), "1W");
}
