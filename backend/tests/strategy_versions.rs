use std::collections::BTreeMap;

use alphapulse_okx_backend::{
    domain::{Direction, Score, SymbolSnapshot},
    paper::{PaperOrderRequest, PaperSide},
    strategy::{
        attribution_by_signal, AttributionConfidence, AttributionSuggestion, AttributionTrade,
        MarketRiskSnapshot, OrderAction, OrderIntent, RiskAccountContext, RiskGuard,
        RiskGuardDecision, StopLossRecord, VersionedPaperState, V3_VERSION_CODE, V4_VERSION_CODE,
    },
};

fn intent(
    version_code: &str,
    run_id: &str,
    symbol: &str,
    side: PaperSide,
    primary_signal: &str,
    score: u8,
    tags: &[&str],
) -> OrderIntent {
    OrderIntent {
        version_code: version_code.to_string(),
        run_id: run_id.to_string(),
        symbol: symbol.to_string(),
        side,
        action: OrderAction::Open,
        margin: 100.0,
        leverage: 10.0,
        score,
        primary_signal: primary_signal.to_string(),
        reason: format!("{primary_signal} score {score}"),
        tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
        stop_loss: Some(9.0),
        take_profit: Some(12.0),
        expire_at: None,
        risk_flags: Vec::new(),
        risk_guard_decision: None,
        config_hash: "test-config".to_string(),
    }
}

fn prices(inst_id: &str, price: f64) -> BTreeMap<String, SymbolSnapshot> {
    let mut prices = BTreeMap::new();
    prices.insert(
        inst_id.to_string(),
        SymbolSnapshot {
            inst_id: inst_id.to_string(),
            price,
            change_5m_pct: 0.0,
            change_15m_pct: 0.0,
            change_1h_pct: 0.0,
            change_24h_pct: None,
            change_48h_pct: None,
            change_72h_pct: None,
            intraday_low_break_count: 0,
            high_volatility_flag: false,
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
            pool_tags: Vec::new(),
            trigger_reason: String::new(),
            funding_rate: None,
            fvgs: Vec::new(),
            levels: Vec::new(),
            updated_at_ms: 1,
        },
    );
    prices
}

fn auto_open_symbol(inst_id: &str, price: f64) -> SymbolSnapshot {
    SymbolSnapshot {
        inst_id: inst_id.to_string(),
        price,
        change_5m_pct: 0.01,
        change_15m_pct: 0.02,
        change_1h_pct: 0.03,
        change_24h_pct: None,
        change_48h_pct: None,
        change_72h_pct: None,
        intraday_low_break_count: 0,
        high_volatility_flag: false,
        trend_score: Score {
            value: 0,
            direction: Direction::Neutral,
            reasons: Vec::new(),
        },
        range_score: Score {
            value: 90,
            direction: Direction::Long,
            reasons: vec!["clear recent range".to_string()],
        },
        pool_tags: vec!["dynamic".to_string()],
        trigger_reason: format!("{inst_id} range long 90"),
        funding_rate: None,
        fvgs: Vec::new(),
        levels: Vec::new(),
        updated_at_ms: 1,
    }
}

#[test]
fn attribution_keeps_high_pf_with_small_sample_as_insufficient() {
    let trades = (0..5)
        .map(|index| AttributionTrade {
            id: index,
            symbol: "LAB-USDT-SWAP".to_string(),
            side: PaperSide::Long,
            primary_signal: "range_long".to_string(),
            tags: vec!["range_long".to_string(), "altcoin".to_string()],
            net_pnl: 25.0,
            exit_reason: "take_profit".to_string(),
        })
        .collect::<Vec<_>>();

    let rows = attribution_by_signal(&trades);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].sample_count, 5);
    assert_eq!(
        rows[0].confidence,
        AttributionConfidence::InsufficientSample
    );
    assert_eq!(
        rows[0].suggestion,
        AttributionSuggestion::InsufficientSample
    );
}

#[test]
fn attribution_marks_large_profitable_sample_as_quality() {
    let mut trades = Vec::new();
    for index in 0..24 {
        trades.push(AttributionTrade {
            id: index,
            symbol: "BREV-USDT-SWAP".to_string(),
            side: PaperSide::Short,
            primary_signal: "multiday_reversal_short".to_string(),
            tags: vec!["short".to_string(), "score_90_100".to_string()],
            net_pnl: 12.0,
            exit_reason: "take_profit".to_string(),
        });
    }
    for index in 24..30 {
        trades.push(AttributionTrade {
            id: index,
            symbol: "BREV-USDT-SWAP".to_string(),
            side: PaperSide::Short,
            primary_signal: "multiday_reversal_short".to_string(),
            tags: vec!["short".to_string(), "score_90_100".to_string()],
            net_pnl: -5.0,
            exit_reason: "stop_loss".to_string(),
        });
    }

    let rows = attribution_by_signal(&trades);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].sample_count, 30);
    assert_eq!(rows[0].confidence, AttributionConfidence::Medium);
    assert_eq!(rows[0].suggestion, AttributionSuggestion::Quality);
    assert!((rows[0].profit_factor.unwrap() - 9.6).abs() < f64::EPSILON);
}

#[test]
fn v4_extreme_dump_gate_blocks_normal_long_and_records_event() {
    let guard = RiskGuard::default();
    let order = intent(
        V4_VERSION_CODE,
        "v0.1.4-paper-1",
        "LAB-USDT-SWAP",
        PaperSide::Long,
        "range_long",
        88,
        &["long", "range_long", "altcoin"],
    );

    let outcome = guard.evaluate(
        &order,
        &MarketRiskSnapshot {
            symbol: "LAB-USDT-SWAP".to_string(),
            change_24h_pct: Some(-0.31),
            change_48h_pct: Some(-0.55),
            change_72h_pct: Some(-0.66),
            intraday_low_break_count: 3,
            high_volatility_flag: true,
            consecutive_dump_days: 2,
        },
        &RiskAccountContext::default(),
        1_000,
    );

    assert_eq!(outcome.decision, RiskGuardDecision::Blocked);
    assert!(outcome.final_intent.is_none());
    let event = outcome
        .event
        .expect("blocked order should create risk event");
    assert_eq!(event.reason, "blocked_by_extreme_dump_gate");
    assert!(event.risk_flags.contains(&"extreme_dump".to_string()));
    assert!(event.risk_flags.contains(&"ban_normal_long".to_string()));
}

#[test]
fn same_symbol_same_direction_stop_loss_enters_cooldown() {
    let mut guard = RiskGuard::default();
    guard.record_stop_loss(StopLossRecord {
        version_code: V4_VERSION_CODE.to_string(),
        run_id: "v0.1.4-paper-1".to_string(),
        symbol: "LAB-USDT-SWAP".to_string(),
        side: PaperSide::Long,
        ts_ms: 10_000,
    });
    let order = intent(
        V4_VERSION_CODE,
        "v0.1.4-paper-1",
        "LAB-USDT-SWAP",
        PaperSide::Long,
        "pattern_long",
        91,
        &["long", "pattern"],
    );

    let outcome = guard.evaluate(
        &order,
        &MarketRiskSnapshot::for_symbol("LAB-USDT-SWAP"),
        &RiskAccountContext::default(),
        10_000 + 30 * 60_000,
    );

    assert_eq!(outcome.decision, RiskGuardDecision::Blocked);
    assert_eq!(
        outcome.event.expect("cooldown should create event").reason,
        "blocked_by_same_symbol_cooldown"
    );
}

#[test]
fn v4_blocks_adding_to_same_symbol_loser() {
    let guard = RiskGuard::default();
    let order = intent(
        V4_VERSION_CODE,
        "v0.1.4-paper-1",
        "LAB-USDT-SWAP",
        PaperSide::Long,
        "range_long",
        93,
        &["long", "range_long"],
    );

    let outcome = guard.evaluate(
        &order,
        &MarketRiskSnapshot::for_symbol("LAB-USDT-SWAP"),
        &RiskAccountContext {
            same_symbol_unrealized_pnl_pct: Some(-0.18),
            ..RiskAccountContext::default()
        },
        20_000,
    );

    assert_eq!(outcome.decision, RiskGuardDecision::Blocked);
    assert_eq!(
        outcome
            .event
            .expect("no-add-to-loser should create event")
            .reason,
        "blocked_by_no_add_to_loser"
    );
}

#[test]
fn v4_account_kill_switch_blocks_new_entries_after_large_drawdown() {
    let guard = RiskGuard::default();
    let order = intent(
        V4_VERSION_CODE,
        "v0.1.4-paper-1",
        "DOGE-USDT-SWAP",
        PaperSide::Short,
        "trend_short",
        95,
        &["short", "trend"],
    );

    let outcome = guard.evaluate(
        &order,
        &MarketRiskSnapshot::for_symbol("DOGE-USDT-SWAP"),
        &RiskAccountContext {
            account_equity: 6_900.0,
            account_peak_equity: 10_000.0,
            ..RiskAccountContext::default()
        },
        30_000,
    );

    assert_eq!(outcome.decision, RiskGuardDecision::Blocked);
    assert_eq!(
        outcome
            .event
            .expect("kill switch should create event")
            .reason,
        "blocked_by_account_kill_switch"
    );
}

#[test]
fn reset_one_version_keeps_other_version_account_intact() {
    let mut state = VersionedPaperState::default();
    let prices = prices("LAB-USDT-SWAP", 10.0);

    state
        .open_order(
            V3_VERSION_CODE,
            PaperOrderRequest {
                inst_id: "LAB-USDT-SWAP".to_string(),
                side: PaperSide::Long,
                margin: 100.0,
                leverage: 10.0,
                ..PaperOrderRequest::default()
            },
            &prices,
            1_000,
        )
        .unwrap();
    let v4_before = state
        .open_order(
            V4_VERSION_CODE,
            PaperOrderRequest {
                inst_id: "LAB-USDT-SWAP".to_string(),
                side: PaperSide::Long,
                margin: 100.0,
                leverage: 10.0,
                ..PaperOrderRequest::default()
            },
            &prices,
            1_000,
        )
        .unwrap()
        .run
        .run_id;

    state.reset_version(V4_VERSION_CODE, 2_000).unwrap();

    let v3 = state.version_snapshot(V3_VERSION_CODE, &prices).unwrap();
    let v4 = state.version_snapshot(V4_VERSION_CODE, &prices).unwrap();
    assert_eq!(v3.paper.positions.len(), 1);
    assert_eq!(v3.paper.positions[0].version_code, V3_VERSION_CODE);
    assert!(v4.paper.positions.is_empty());
    assert_ne!(v4.run.run_id, v4_before);
}

#[test]
fn auto_strategy_caps_open_positions_per_version_at_five() {
    let mut state = VersionedPaperState::default();
    let mut prices = BTreeMap::new();

    for index in 0..6 {
        let symbol = auto_open_symbol(&format!("CAP-{index}-USDT-SWAP"), 10.0 + index as f64);
        prices.insert(symbol.inst_id.clone(), symbol.clone());
        state.process_market_update(&symbol, &prices, 1_000 + index as i64);
    }

    let v3 = state.version_snapshot(V3_VERSION_CODE, &prices).unwrap();
    let v4 = state.version_snapshot(V4_VERSION_CODE, &prices).unwrap();
    assert_eq!(v3.paper.positions.len(), 5);
    assert_eq!(v4.paper.positions.len(), 5);
}
