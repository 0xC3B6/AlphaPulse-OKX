use std::collections::BTreeMap;

use alphapulse_okx_backend::{
    config::AppConfig,
    paper::{PaperOrderRequest, PaperSide, PaperState},
    persistence::{postgres_schema_statements, PersistenceHealthSnapshot, PersistenceStatus},
};

#[test]
fn production_config_requires_database_and_reads_redis() {
    let config = AppConfig::from_env_pairs([
        ("ALPHAPULSE_DATABASE_URL", "postgres://u:p@127.0.0.1/db"),
        ("ALPHAPULSE_REDIS_URL", "redis://127.0.0.1:6379/"),
        ("ALPHAPULSE_REQUIRE_DATABASE", "true"),
    ]);
    assert_eq!(
        config.database_url.as_deref(),
        Some("postgres://u:p@127.0.0.1/db")
    );
    assert_eq!(config.redis_url.as_deref(), Some("redis://127.0.0.1:6379/"));
    assert!(config.require_database);
}

#[test]
fn schema_persists_strategy_identity_and_protective_levels() {
    let schema = postgres_schema_statements().join("\n");
    for required in [
        "strategy_build_id",
        "config_hash",
        "order_intents",
        "fills",
        "positions",
        "stop_loss",
        "take_profit",
        "closed_trades",
        "equity_snapshots",
        "event_log",
        "app_state_snapshots",
    ] {
        assert!(schema.contains(required), "missing {required}");
    }
    assert!(!schema.contains("CREATE TABLE IF NOT EXISTS risk_guard_events"));
}

#[test]
fn paper_checkpoint_round_trips_open_and_closed_positions() {
    let mut state = PaperState::default();
    state
        .open(
            automatic("ETH-USDT-SWAP", PaperSide::Long, 1_000.0),
            1_000.0,
            10_000.0,
            1,
        )
        .unwrap();
    state
        .close_with_meta("ETH-USDT-SWAP", 1_020.0, 2, "auto", "take profit")
        .unwrap();
    state
        .open(
            automatic("SOL-USDT-SWAP", PaperSide::Short, 100.0),
            100.0,
            10_000.0,
            3,
        )
        .unwrap();

    let encoded = serde_json::to_value(&state).unwrap();
    let restored: PaperState = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), encoded);
    let snapshot = restored.snapshot(&BTreeMap::from([
        ("ETH-USDT-SWAP".to_string(), 1_020.0),
        ("SOL-USDT-SWAP".to_string(), 100.0),
    ]));
    assert_eq!(snapshot.positions.len(), 1);
    assert_eq!(snapshot.position_history.len(), 1);
    let stop_loss = snapshot.positions[0].stop_loss.unwrap();
    assert!((stop_loss - 101.5).abs() < 1e-9);
}

#[test]
fn persistence_health_has_explicit_healthy_and_paused_states() {
    assert_eq!(
        PersistenceHealthSnapshot::healthy(42),
        PersistenceHealthSnapshot {
            status: PersistenceStatus::Healthy,
            last_committed_at_ms: Some(42),
            last_error: None,
        }
    );
    let paused = PersistenceHealthSnapshot::paused("database unavailable");
    assert_eq!(paused.status, PersistenceStatus::PersistencePaused);
    assert_eq!(paused.last_error.as_deref(), Some("database unavailable"));
}

fn automatic(inst_id: &str, side: PaperSide, price: f64) -> PaperOrderRequest {
    let direction = match side {
        PaperSide::Long => 1.0,
        PaperSide::Short => -1.0,
    };
    PaperOrderRequest::automatic(
        inst_id,
        side,
        300.0,
        20.0,
        price * (1.0 + direction * -0.30 / 20.0),
        price * (1.0 + direction * 0.40 / 20.0),
        None,
        match side {
            PaperSide::Long => "trend_long",
            PaperSide::Short => "trend_short",
        },
        "persistence fixture",
        vec!["persistence".to_string()],
    )
}
