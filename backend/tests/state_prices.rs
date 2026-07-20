use alphapulse_okx_backend::{
    auto_strategy::AutoStrategyConfig,
    domain::{Direction, Score, SymbolSnapshot},
    paper::{PaperOrderRequest, PaperSide},
    state::RadarState,
};

#[tokio::test]
async fn paper_uses_latest_price_even_when_symbol_is_not_in_radar_snapshot() {
    let state = RadarState::default();
    assert!(state
        .update_symbol_price("BIO-USDT-SWAP", 0.030156, 1_000)
        .await
        .is_none());
    let opened = state
        .open_paper_order(PaperOrderRequest::manual(
            "BIO-USDT-SWAP",
            PaperSide::Long,
            300.0,
            20.0,
        ))
        .await
        .unwrap();
    assert_eq!(opened.positions.len(), 1);

    state
        .update_symbol_price("BIO-USDT-SWAP", 0.030500, 2_000)
        .await;
    let paper = state.paper_snapshot().await;
    assert!(state.snapshot().await.symbols.is_empty());
    assert!((paper.positions[0].mark_price - 0.030500).abs() < f64::EPSILON);
    assert!(paper.positions[0].unrealized_pnl > 0.0);
}

#[tokio::test]
async fn ticker_sync_ids_include_open_positions_outside_radar_pool() {
    let state = RadarState::default();
    state
        .update_symbol_price("BIO-USDT-SWAP", 0.030156, 1_000)
        .await;
    state
        .open_paper_order(PaperOrderRequest::manual(
            "BIO-USDT-SWAP",
            PaperSide::Long,
            300.0,
            20.0,
        ))
        .await
        .unwrap();
    let ids = state
        .ticker_sync_inst_ids(&["BTC-USDT-SWAP".to_string(), "BIO-USDT-SWAP".to_string()])
        .await;
    assert_eq!(ids, vec!["BIO-USDT-SWAP", "BTC-USDT-SWAP"]);
}

#[tokio::test]
async fn latest_price_update_auto_closes_stop_loss_outside_radar_pool() {
    let state = seeded_state_with_long("CRV-USDT-SWAP", 1.0, 300.0, 20.0).await;
    state
        .update_symbol_price("CRV-USDT-SWAP", 0.90, 2_000)
        .await;
    let paper = state.paper_snapshot().await;
    assert!(paper.positions.is_empty());
    assert_eq!(paper.position_history.len(), 1);
    assert!(paper.position_history[0].close_reason.contains("stop loss"));
}

#[tokio::test]
async fn stale_ticker_timestamp_does_not_backdate_auto_close() {
    let state = seeded_state_with_long("EDGE-USDT-SWAP", 1.0, 300.0, 20.0).await;
    let opened_at_ms = state.paper_snapshot().await.positions[0].opened_at_ms;
    state
        .update_symbol_price("EDGE-USDT-SWAP", 0.90, 1_000)
        .await;
    let closed = &state.paper_snapshot().await.position_history[0];
    assert!(closed.closed_at_ms >= opened_at_ms);
    assert!(closed.duration_ms >= 0);
}

#[tokio::test]
async fn batched_price_update_evaluates_auto_exit_before_new_entry() {
    let state = RadarState::default();
    for inst_id in ["A", "B", "C", "D", "E"] {
        let inst_id = format!("{inst_id}-USDT-SWAP");
        state.update_symbol_price(&inst_id, 1.0, 1).await;
        state
            .open_paper_order(automatic_order(&inst_id, PaperSide::Long, 1.0, 100.0, 20.0))
            .await
            .unwrap();
    }
    state
        .upsert_symbol(symbol(
            "NEW-USDT-SWAP",
            2.0,
            score(100, Direction::Long),
            score(0, Direction::Neutral),
        ))
        .await;

    state
        .update_latest_prices(vec![
            ("A-USDT-SWAP".to_string(), 0.90, 2),
            ("NEW-USDT-SWAP".to_string(), 2.0, 2),
        ])
        .await;
    let paper = state.paper_snapshot().await;
    assert!(paper
        .positions
        .iter()
        .all(|position| position.inst_id != "A-USDT-SWAP"));
    assert!(paper
        .positions
        .iter()
        .any(|position| position.inst_id == "NEW-USDT-SWAP"));
    assert!(paper.position_history[0].close_reason.contains("stop loss"));
}

#[tokio::test]
async fn crossed_stop_uses_stored_trigger_price_then_one_slippage_application() {
    let state = seeded_state_with_long("CRV-USDT-SWAP", 1.0, 300.0, 20.0).await;
    state
        .update_latest_prices(vec![("CRV-USDT-SWAP".to_string(), 0.90, 2)])
        .await;
    let paper = state.paper_snapshot().await;
    let history = &paper.position_history[0];
    assert_eq!(history.stop_loss, Some(0.985));
    assert_close(history.exit_price, 0.985 * (1.0 - paper.slippage_rate));
}

#[tokio::test]
async fn long_and_short_protective_boundaries_close_at_stored_levels() {
    let long_state = seeded_state_with_long("LONG-USDT-SWAP", 1.0, 300.0, 20.0).await;
    long_state
        .update_latest_prices(vec![("LONG-USDT-SWAP".to_string(), 1.05, 2)])
        .await;
    let long_paper = long_state.paper_snapshot().await;
    assert_eq!(long_paper.position_history[0].take_profit, Some(1.02));
    assert!(long_paper.position_history[0]
        .close_reason
        .contains("take profit"));
    assert_close(
        long_paper.position_history[0].exit_price,
        1.02 * (1.0 - long_paper.slippage_rate),
    );

    let short_state = seeded_state_with_short("SHORT-USDT-SWAP", 1.0, 300.0, 20.0).await;
    short_state
        .update_latest_prices(vec![("SHORT-USDT-SWAP".to_string(), 1.10, 2)])
        .await;
    let short_paper = short_state.paper_snapshot().await;
    assert_eq!(short_paper.position_history[0].stop_loss, Some(1.015));
    assert!(short_paper.position_history[0]
        .close_reason
        .contains("stop loss"));
    assert_close(
        short_paper.position_history[0].exit_price,
        1.015 * (1.0 + short_paper.slippage_rate),
    );
}

#[tokio::test]
async fn no_crossed_protective_level_remains_as_a_minus_forty_percent_position() {
    let state = seeded_state_with_long("LOSS-USDT-SWAP", 1.0, 300.0, 20.0).await;
    state
        .update_latest_prices(vec![("LOSS-USDT-SWAP".to_string(), 0.97, 2)])
        .await;
    let paper = state.paper_snapshot().await;
    assert!(paper.positions.is_empty());
    assert_eq!(paper.position_history.len(), 1);
}

#[tokio::test]
async fn direct_auto_run_preserves_v3_trade_metadata_and_tags() {
    let state = RadarState::default();
    let symbol = symbol(
        "ETH-USDT-SWAP",
        1_600.0,
        score(100, Direction::Long),
        score(0, Direction::Neutral),
    );
    state.upsert_symbol(symbol.clone()).await;
    let paper = state
        .run_auto_strategy_for_symbol_at(
            &symbol,
            AutoStrategyConfig::default(),
            chrono::DateTime::parse_from_rfc3339("2026-07-02T13:35:00Z")
                .unwrap()
                .timestamp_millis(),
        )
        .await
        .unwrap()
        .unwrap();
    assert_eq!(paper.positions.len(), 1);
    assert_eq!(paper.positions[0].strategy_version, "v0.1.3");
    assert_eq!(paper.positions[0].primary_signal, "trend_long");
    assert!(!paper.positions[0].tags.is_empty());
}

async fn seeded_state_with_long(
    inst_id: &str,
    price: f64,
    margin: f64,
    leverage: f64,
) -> RadarState {
    seeded_state(inst_id, price, margin, leverage, PaperSide::Long).await
}

async fn seeded_state_with_short(
    inst_id: &str,
    price: f64,
    margin: f64,
    leverage: f64,
) -> RadarState {
    seeded_state(inst_id, price, margin, leverage, PaperSide::Short).await
}

async fn seeded_state(
    inst_id: &str,
    price: f64,
    margin: f64,
    leverage: f64,
    side: PaperSide,
) -> RadarState {
    let state = RadarState::default();
    state.update_symbol_price(inst_id, price, 1).await;
    state
        .open_paper_order(automatic_order(inst_id, side, price, margin, leverage))
        .await
        .unwrap();
    state
}

fn automatic_order(
    inst_id: &str,
    side: PaperSide,
    price: f64,
    margin: f64,
    leverage: f64,
) -> PaperOrderRequest {
    let direction = match side {
        PaperSide::Long => 1.0,
        PaperSide::Short => -1.0,
    };
    PaperOrderRequest::automatic(
        inst_id,
        side,
        margin,
        leverage,
        price * (1.0 + direction * -0.30 / leverage),
        price * (1.0 + direction * 0.40 / leverage),
        None,
        match side {
            PaperSide::Long => "trend_long",
            PaperSide::Short => "trend_short",
        },
        "state price fixture",
        Vec::new(),
    )
}

fn symbol(inst_id: &str, price: f64, trend_score: Score, range_score: Score) -> SymbolSnapshot {
    SymbolSnapshot {
        inst_id: inst_id.to_string(),
        price,
        change_5m_pct: 0.0,
        change_15m_pct: 0.0,
        change_1h_pct: 0.0,
        trend_score,
        range_score,
        pool_tags: vec!["dynamic".to_string()],
        trigger_reason: String::new(),
        funding_rate: None,
        scalping_metrics: Default::default(),
        fvgs: Vec::new(),
        levels: Vec::new(),
        pattern_signals: Vec::new(),
        updated_at_ms: 1,
    }
}

fn score(value: u8, direction: Direction) -> Score {
    Score {
        value,
        direction,
        reasons: Vec::new(),
    }
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {actual} to be near {expected}"
    );
}
