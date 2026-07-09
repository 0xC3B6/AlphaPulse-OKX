use alphapulse_okx_backend::{
    persistence::{decimal_from_f64, postgres_schema_statements, PersistenceLayer},
    strategy::{VersionedPaperState, V3_VERSION_CODE, V4_VERSION_CODE},
};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[test]
fn postgres_schema_covers_required_trading_tables() {
    let schema = postgres_schema_statements().join("\n");

    for table in [
        "strategy_versions",
        "strategy_runs",
        "order_intents",
        "fills",
        "positions",
        "closed_trades",
        "equity_snapshots",
        "risk_guard_events",
        "event_log",
        "app_state_snapshots",
    ] {
        assert!(schema.contains(&format!("CREATE TABLE IF NOT EXISTS {table}")));
    }
}

#[test]
fn persistence_layer_can_be_disabled_for_local_tests() {
    let layer = PersistenceLayer::disabled();

    assert!(!layer.is_postgres_enabled());
    assert!(!layer.is_redis_enabled());
}

#[test]
fn decimal_conversion_rejects_non_finite_values() {
    assert_eq!(decimal_from_f64(12.3456).unwrap(), Decimal::new(123456, 4));
    assert!(decimal_from_f64(f64::NAN).is_none());
    assert!(decimal_from_f64(f64::INFINITY).is_none());
}

#[test]
fn versioned_paper_state_can_roundtrip_for_snapshot_restore() {
    let state = VersionedPaperState::default();
    let json = serde_json::to_string(&state).unwrap();
    let restored: VersionedPaperState = serde_json::from_str(&json).unwrap();

    assert!(restored
        .version_snapshot(V3_VERSION_CODE, &BTreeMap::new())
        .is_ok());
    assert!(restored
        .version_snapshot(V4_VERSION_CODE, &BTreeMap::new())
        .is_ok());
}
