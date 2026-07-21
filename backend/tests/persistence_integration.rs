use std::collections::BTreeMap;

use alphapulse_okx_backend::{
    config::AppConfig,
    domain::{Direction, Score, SymbolSnapshot},
    paper::{PaperOrderRequest, PaperSide, PaperState},
    persistence::{PersistedOrderIntent, PersistedTransition, PersistenceLayer},
    server,
    state::RadarState,
    strategy_identity::{StrategyIdentity, INITIAL_RUN_ID},
};
use redis::AsyncCommands;

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn two_tenants_can_persist_the_same_strategy_run_without_key_collisions() {
    let database_url = std::env::var("ALPHAPULSE_TEST_DATABASE_URL").unwrap();
    let redis_url = std::env::var("ALPHAPULSE_TEST_REDIS_URL").unwrap();
    let left_config = AppConfig::from_env_pairs([
        ("ALPHAPULSE_DATABASE_URL", database_url.as_str()),
        ("ALPHAPULSE_REDIS_URL", redis_url.as_str()),
        ("ALPHAPULSE_TENANT_ID", "tenant-left"),
        ("ALPHAPULSE_ACCOUNT_ID", "paper"),
    ]);
    let right_config = AppConfig::from_env_pairs([
        ("ALPHAPULSE_DATABASE_URL", database_url.as_str()),
        ("ALPHAPULSE_REDIS_URL", redis_url.as_str()),
        ("ALPHAPULSE_TENANT_ID", "tenant-right"),
        ("ALPHAPULSE_ACCOUNT_ID", "paper"),
    ]);
    let left = PersistenceLayer::connect_required(&left_config)
        .await
        .unwrap();
    let right = PersistenceLayer::connect_required(&right_config)
        .await
        .unwrap();
    left.initialize().await.unwrap();
    left.purge_strategy_data(&["v0.1.3", "v0.1.4"])
        .await
        .unwrap();
    let paper = PaperState::fresh_restored_v3(StrategyIdentity::restored_v3());
    let snapshot = paper.snapshot(&BTreeMap::<String, f64>::new());

    left.persist_checkpoint(&paper, &snapshot, 1).await.unwrap();
    right
        .persist_checkpoint(&paper, &snapshot, 1)
        .await
        .unwrap();

    let pool = sqlx::PgPool::connect(&database_url).await.unwrap();
    let run_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM strategy_runs WHERE run_id = ANY($1)")
            .bind(vec![
                left.database_run_id(INITIAL_RUN_ID),
                right.database_run_id(INITIAL_RUN_ID),
            ])
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(run_count, 2);
    let snapshot_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM app_state_snapshots WHERE tenant_id = ANY($1) AND account_id = 'paper'",
    )
    .bind(vec!["tenant-left", "tenant-right"])
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(snapshot_count, 2);
}

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn transition_locks_account_advances_version_and_writes_ledger() {
    let config = test_config(true);
    let persistence = PersistenceLayer::connect_required(&config).await.unwrap();
    persistence.initialize().await.unwrap();
    persistence
        .purge_strategy_data(&["v0.1.3", "v0.1.4"])
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(config.database_url.as_deref().unwrap())
        .await
        .unwrap();
    let scope = persistence.account_scope();
    let version_before = sqlx::query_scalar::<_, i64>(
        "SELECT account_version FROM trading_accounts WHERE tenant_id = $1 AND account_id = $2",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .fetch_optional(&pool)
    .await
    .unwrap()
    .unwrap_or(0);

    let mut candidate = PaperState::fresh_restored_v3(StrategyIdentity::restored_v3());
    let order = automatic(
        "LEDGER-USDT-SWAP",
        PaperSide::Long,
        98.5,
        102.0,
        "trend_long",
    );
    let trade = candidate.open(order.clone(), 100.0, 10_000.0, 10).unwrap();
    let snapshot = candidate.snapshot(&BTreeMap::from([("LEDGER-USDT-SWAP".to_string(), 100.0)]));
    persistence
        .persist_transition(&PersistedTransition {
            event_type: "paper_open".to_string(),
            intent: Some(PersistedOrderIntent::accepted_open(
                &candidate, &order, &trade, 90,
            )),
            state: candidate,
            snapshot,
            committed_at_ms: 10,
        })
        .await
        .unwrap();

    let version_after: i64 = sqlx::query_scalar(
        "SELECT account_version FROM trading_accounts WHERE tenant_id = $1 AND account_id = $2",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(version_after, version_before + 1);
    let ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ledger_entries WHERE tenant_id = $1 AND account_id = $2 AND run_id = $3",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .bind(persistence.database_run_id(INITIAL_RUN_ID))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ledger_count, 1);
    let logged_version: i64 = sqlx::query_scalar(
        "SELECT account_version FROM event_log WHERE tenant_id = $1 AND account_id = $2 ORDER BY id DESC LIMIT 1",
    )
    .bind(&scope.tenant_id)
    .bind(&scope.account_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(logged_version, version_after);
}

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn restart_restores_balance_positions_protection_history_and_ids() {
    let config = test_config(true);
    let persistence = PersistenceLayer::connect_required(&config).await.unwrap();
    persistence.initialize().await.unwrap();
    persistence
        .purge_strategy_data(&["v0.1.3", "v0.1.4"])
        .await
        .unwrap();

    let identity = StrategyIdentity::restored_v3();
    let mut original = PaperState::fresh_restored_v3(identity.clone());
    original
        .open(
            automatic(
                "BTC-USDT-SWAP",
                PaperSide::Long,
                49_250.0,
                51_000.0,
                "trend_long",
            ),
            50_000.0,
            10_000.0,
            1,
        )
        .unwrap();
    original
        .close_with_meta("BTC-USDT-SWAP", 51_000.0, 2, "auto", "take profit")
        .unwrap();
    original
        .open(
            automatic(
                "ETH-USDT-SWAP",
                PaperSide::Long,
                985.0,
                1_020.0,
                "pattern_long",
            ),
            1_000.0,
            10_000.0,
            3,
        )
        .unwrap();
    let prices = BTreeMap::from([
        (
            "BTC-USDT-SWAP".to_string(),
            symbol("BTC-USDT-SWAP", 51_000.0),
        ),
        (
            "ETH-USDT-SWAP".to_string(),
            symbol("ETH-USDT-SWAP", 1_000.0),
        ),
    ]);
    let original_snapshot = original.snapshot(&prices);
    persistence
        .persist_checkpoint(&original, &original_snapshot, 100)
        .await
        .unwrap();
    persistence.clear_cache().await.unwrap();

    let equity_history = persistence
        .load_equity_history(&identity, original.run_id())
        .await
        .unwrap();
    assert_eq!(equity_history.len(), 1);
    assert_eq!(equity_history[0].timestamp_ms, 100);
    assert_persisted_decimal_eq(equity_history[0].equity, original_snapshot.equity);
    assert_persisted_decimal_eq(
        equity_history[0].unrealized_pnl,
        original_snapshot.unrealized_pnl,
    );

    let restored = persistence
        .load_paper_state(&identity)
        .await
        .unwrap()
        .expect("persisted state");
    let restored_snapshot = restored.snapshot(&prices);

    assert_eq!(
        restored_snapshot.realized_pnl,
        original_snapshot.realized_pnl
    );
    assert_eq!(restored_snapshot.total_fees, original_snapshot.total_fees);
    assert_eq!(restored.next_trade_id(), original.next_trade_id());
    assert_eq!(restored_snapshot.positions[0].stop_loss, Some(985.0));
    assert_eq!(restored_snapshot.positions[0].take_profit, Some(1_020.0));
    assert_eq!(
        restored_snapshot.position_history.len(),
        original_snapshot.position_history.len()
    );
}

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn failed_fill_rolls_back_intent_position_and_checkpoint() {
    let config = test_config(true);
    let persistence = PersistenceLayer::connect_required(&config).await.unwrap();
    persistence.initialize().await.unwrap();
    persistence
        .purge_strategy_data(&["v0.1.3", "v0.1.4"])
        .await
        .unwrap();
    let pool = sqlx::PgPool::connect(config.database_url.as_deref().unwrap())
        .await
        .unwrap();
    sqlx::query(
        "CREATE OR REPLACE FUNCTION fail_test_fill() RETURNS trigger AS $$ BEGIN RAISE EXCEPTION 'forced fill failure'; END; $$ LANGUAGE plpgsql",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("CREATE TRIGGER fail_test_fill BEFORE INSERT ON fills FOR EACH ROW EXECUTE FUNCTION fail_test_fill()")
        .execute(&pool)
        .await
        .unwrap();

    let mut candidate = PaperState::fresh_restored_v3(StrategyIdentity::restored_v3());
    let order = automatic(
        "ROLLBACK-USDT-SWAP",
        PaperSide::Long,
        98.5,
        102.0,
        "trend_long",
    );
    let trade = candidate.open(order.clone(), 100.0, 10_000.0, 10).unwrap();
    let snapshot = candidate.snapshot(&BTreeMap::from([("ROLLBACK-USDT-SWAP".to_string(), 100.0)]));
    let transition = PersistedTransition {
        event_type: "paper_open".to_string(),
        intent: Some(PersistedOrderIntent::accepted_open(
            &candidate, &order, &trade, 90,
        )),
        state: candidate,
        snapshot,
        committed_at_ms: 10,
    };
    assert!(persistence.persist_transition(&transition).await.is_err());

    sqlx::query("DROP TRIGGER fail_test_fill ON fills")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP FUNCTION fail_test_fill()")
        .execute(&pool)
        .await
        .unwrap();
    for table in ["order_intents", "fills", "positions", "app_state_snapshots"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "{table} was not rolled back");
    }
}

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn clearing_redis_can_be_rebuilt_from_committed_dashboard() {
    let config = test_config(true);
    let redis_url = config.redis_url.clone().unwrap();
    let persistence = PersistenceLayer::connect_required(&config).await.unwrap();
    persistence.initialize().await.unwrap();
    let paper = PaperState::fresh_restored_v3(StrategyIdentity::restored_v3());
    persistence
        .persist_checkpoint(&paper, &paper.snapshot(&BTreeMap::<String, f64>::new()), 20)
        .await
        .unwrap();
    let state = RadarState::with_persistence(persistence.clone(), paper);
    let dashboard = state.snapshot().await;
    persistence.clear_cache().await.unwrap();
    persistence.rebuild_cache(&dashboard).await.unwrap();

    let client = redis::Client::open(redis_url).unwrap();
    let mut connection = client.get_multiplexed_async_connection().await.unwrap();
    let payload: String = connection
        .get(persistence.account_scope().redis_key("dashboard:snapshot"))
        .await
        .unwrap();
    let cached: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert_eq!(cached["paper"]["initial_balance"], 10_000.0);
}

#[tokio::test]
#[ignore = "requires docker compose PostgreSQL and Redis"]
async fn redis_failure_does_not_undo_postgres_transition() {
    let database_url = std::env::var("ALPHAPULSE_TEST_DATABASE_URL").unwrap();
    let config = AppConfig::from_env_pairs([
        ("ALPHAPULSE_DATABASE_URL", database_url.as_str()),
        ("ALPHAPULSE_REDIS_URL", "redis://127.0.0.1:1/"),
        ("ALPHAPULSE_REQUIRE_DATABASE", "true"),
    ]);
    let persistence = PersistenceLayer::connect_required(&config).await.unwrap();
    persistence.initialize().await.unwrap();
    persistence
        .purge_strategy_data(&["v0.1.3", "v0.1.4"])
        .await
        .unwrap();
    let paper = PaperState::fresh_restored_v3(StrategyIdentity::restored_v3());
    persistence
        .persist_checkpoint(&paper, &paper.snapshot(&BTreeMap::<String, f64>::new()), 30)
        .await
        .unwrap();
    let state = RadarState::with_persistence(persistence.clone(), paper);
    assert!(persistence
        .rebuild_cache(&state.snapshot().await)
        .await
        .is_err());
    assert!(persistence
        .load_paper_state(&StrategyIdentity::restored_v3())
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
#[ignore = "requires a deliberately unavailable PostgreSQL endpoint"]
async fn server_initialization_fails_before_scanner_when_postgres_is_down() {
    let config = AppConfig::from_env_pairs([
        (
            "ALPHAPULSE_DATABASE_URL",
            "postgres://alphapulse:alphapulse@127.0.0.1:1/alphapulse",
        ),
        ("ALPHAPULSE_REQUIRE_DATABASE", "true"),
    ]);
    assert!(server::initialize_state(&config).await.is_err());
}

fn test_config(with_redis: bool) -> AppConfig {
    let database_url = std::env::var("ALPHAPULSE_TEST_DATABASE_URL").unwrap();
    let redis_url = std::env::var("ALPHAPULSE_TEST_REDIS_URL").unwrap();
    let mut pairs = vec![
        ("ALPHAPULSE_DATABASE_URL", database_url.as_str()),
        ("ALPHAPULSE_REQUIRE_DATABASE", "true"),
    ];
    if with_redis {
        pairs.push(("ALPHAPULSE_REDIS_URL", redis_url.as_str()));
    }
    AppConfig::from_env_pairs(pairs)
}

fn automatic(
    inst_id: &str,
    side: PaperSide,
    stop_loss: f64,
    take_profit: f64,
    primary_signal: &str,
) -> PaperOrderRequest {
    PaperOrderRequest::automatic(
        inst_id,
        side,
        300.0,
        20.0,
        stop_loss,
        take_profit,
        None,
        primary_signal,
        "persistence integration fixture",
        vec!["persistence".to_string()],
    )
}

fn symbol(inst_id: &str, price: f64) -> SymbolSnapshot {
    SymbolSnapshot {
        inst_id: inst_id.to_string(),
        price,
        change_5m_pct: 0.0,
        change_15m_pct: 0.0,
        change_1h_pct: 0.0,
        amplitude_24h_pct: 0.0,
        trend_score: Score {
            value: 0,
            direction: Direction::Neutral,
            reasons: Vec::new(),
        },
        range_score: Score {
            value: 0,
            direction: Direction::Neutral,
            reasons: Vec::new(),
        },
        pool_tags: vec!["integration".to_string()],
        trigger_reason: String::new(),
        funding_rate: None,
        scalping_metrics: Default::default(),
        fvgs: Vec::new(),
        levels: Vec::new(),
        pattern_signals: Vec::new(),
        updated_at_ms: 1,
    }
}

fn assert_persisted_decimal_eq(actual: f64, expected: f64) {
    // Persistence normalizes f64 values to eight decimal places before binding NUMERIC columns.
    assert!(
        (actual - expected).abs() <= 1e-8,
        "persisted value {actual} differs from expected {expected}"
    );
}
