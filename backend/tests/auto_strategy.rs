use alphapulse_okx_backend::{
    auto_strategy::{
        evaluate_auto_exit, evaluate_auto_strategy, evaluate_auto_strategy_at, AutoExitKind,
        AutoStrategyConfig, AutoStrategyDecision,
    },
    domain::{
        Direction, PatternKind, PatternLevelZone, PatternSignal, PatternStatus, ScalpingMetrics,
        Score, SymbolSnapshot, Timeframe,
    },
    market_context::{INTRADAY_UP_EXTREME, MULTIDAY_UP_EXTREME},
    paper::{
        PaperAccountSnapshot, PaperPositionSnapshot, PaperSide, SCALPING_OPTIMIZATION_NAME,
        SCALPING_OPTIMIZATION_SOURCE, SCALPING_OPTIMIZATION_VERSION,
    },
    persistence::PersistenceHealthSnapshot,
    strategy_identity::{StrategyIdentity, INITIAL_RUN_ID, STRATEGY_BUILD_ID},
    time_regime::TradeTagKind,
};

#[test]
fn default_config_matches_the_restored_v3_contract() {
    let config = AutoStrategyConfig::default();
    assert!(config.enabled);
    assert_eq!(config.default_leverage, 20.0);
    assert_eq!(config.margin_fraction, 0.03);
    assert_eq!(config.max_positions, 5);
    assert_eq!(config.stop_loss_margin_pct, -0.30);
    assert_eq!(config.take_profit_margin_pct, 0.40);
    assert!(config.allow_multiday_reversal_short);
    assert!(config.allow_trend_long && config.allow_trend_short);
    assert!(config.allow_range_long && config.allow_range_short);
    assert!(config.allow_pattern_long && config.allow_pattern_short);
    assert!(config.allow_mover_long && config.allow_mover_short);
    assert_eq!(config.trend_threshold, 80);
    assert_eq!(config.range_threshold, 85);
    assert_eq!(config.pattern_threshold, 75);
    let restored: AutoStrategyConfig =
        serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
    assert_eq!(restored, config);
}

#[test]
fn opens_high_trend_long_with_default_risk() {
    let symbol = symbol(
        "ETH-USDT-SWAP",
        1_600.0,
        score(82, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .unwrap();

    match decision {
        AutoStrategyDecision::Open { order, reason, .. } => {
            assert_eq!(order.inst_id, "ETH-USDT-SWAP");
            assert_eq!(order.side, PaperSide::Long);
            assert_eq!(order.leverage, 20.0);
            assert!((order.margin - 90.0).abs() < f64::EPSILON);
            assert_eq!(order.stop_loss, Some(1_576.0));
            assert_eq!(order.take_profit, Some(1_632.0));
            assert_eq!(order.primary_signal.as_deref(), Some("trend_long"));
            assert!(reason.contains("trend long 82"));
        }
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn does_not_open_duplicate_symbol_position() {
    let symbol = symbol(
        "ETH-USDT-SWAP",
        1_600.0,
        score(88, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    let paper = paper_account(
        10_000.0,
        vec![position("ETH-USDT-SWAP", PaperSide::Long, 0.10)],
    );
    assert!(evaluate_auto_strategy(&symbol, &paper, AutoStrategyConfig::default()).is_none());
}

#[test]
fn does_not_open_when_max_positions_reached() {
    let symbol = symbol(
        "NEW-USDT-SWAP",
        2.0,
        score(90, Direction::Short),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    let paper = paper_account(
        10_000.0,
        vec![
            position("A-USDT-SWAP", PaperSide::Long, 0.0),
            position("B-USDT-SWAP", PaperSide::Long, 0.0),
            position("C-USDT-SWAP", PaperSide::Short, 0.0),
            position("D-USDT-SWAP", PaperSide::Long, 0.0),
            position("E-USDT-SWAP", PaperSide::Short, 0.0),
        ],
    );
    assert!(evaluate_auto_strategy(&symbol, &paper, AutoStrategyConfig::default()).is_none());
}

#[test]
fn opens_high_signal_outside_time_risk_window() {
    let symbol = symbol(
        "ETH-USDT-SWAP",
        1_600.0,
        score(90, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .unwrap();
    match decision {
        AutoStrategyDecision::Open { tags, .. } => assert!(tags.is_empty()),
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn time_risk_window_blocks_sub_threshold_signal_without_direction_bias() {
    let symbol = symbol(
        "ETH-USDT-SWAP",
        1_600.0,
        score(90, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T13:35:00Z"),
    )
    .is_none());
}

#[test]
fn very_strong_signal_can_open_in_time_risk_window_with_tags() {
    let symbol = symbol(
        "ETH-USDT-SWAP",
        1_600.0,
        score(100, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T13:35:00Z"),
    )
    .unwrap();
    match decision {
        AutoStrategyDecision::Open { reason, tags, .. } => {
            assert!(reason.contains("time penalty"));
            assert!(tags
                .iter()
                .any(|tag| tag.kind == TradeTagKind::TimeRiskUsOpen));
            assert!(tags
                .iter()
                .any(|tag| tag.kind == TradeTagKind::RequiresHighConfidence));
        }
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn intraday_up_extension_blocks_ordinary_chase_long() {
    let symbol = symbol(
        "BASED-USDT-SWAP",
        0.25,
        score(89, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic", INTRADAY_UP_EXTREME],
    );
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn multiday_up_extension_short_is_smaller_probe_with_tags() {
    let symbol = symbol(
        "BASED-USDT-SWAP",
        0.25,
        score(90, Direction::Short),
        score(20, Direction::Neutral),
        vec!["dynamic", MULTIDAY_UP_EXTREME],
    );
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .unwrap();
    match decision {
        AutoStrategyDecision::Open {
            order,
            reason,
            tags,
        } => {
            assert_eq!(order.side, PaperSide::Short);
            assert!((order.margin - 180.0).abs() < f64::EPSILON);
            assert!(reason.contains("multiday extension reversal short 90"));
            assert!(tags
                .iter()
                .any(|tag| tag.kind == TradeTagKind::OverextensionReversalProbe));
        }
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn multiday_up_extension_midday_reversal_opens_short_probe() {
    let mut symbol = symbol(
        "BASED-USDT-SWAP",
        0.136,
        score(68, Direction::Short),
        score(20, Direction::Neutral),
        vec!["dynamic", MULTIDAY_UP_EXTREME],
    );
    symbol.change_15m_pct = -0.045;
    symbol.change_1h_pct = -0.035;
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T06:30:00Z"),
    )
    .unwrap();
    match decision {
        AutoStrategyDecision::Open { order, tags, .. } => {
            assert_eq!(order.side, PaperSide::Short);
            assert!((order.margin - 180.0).abs() < f64::EPSILON);
            assert!(tags
                .iter()
                .any(|tag| tag.kind == TradeTagKind::MiddayReversalWindow));
        }
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn multiday_up_extension_reversal_can_open_outside_midday_window() {
    let mut symbol = symbol(
        "BASED-USDT-SWAP",
        0.136,
        score(68, Direction::Short),
        score(20, Direction::Neutral),
        vec!["dynamic", MULTIDAY_UP_EXTREME],
    );
    symbol.change_15m_pct = -0.045;
    symbol.change_1h_pct = -0.035;
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .unwrap();
    match decision {
        AutoStrategyDecision::Open { tags, .. } => assert!(tags
            .iter()
            .all(|tag| tag.kind != TradeTagKind::MiddayReversalWindow)),
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn multiday_up_extension_ordinary_pullback_does_not_open_probe() {
    let mut symbol = symbol(
        "BREV-USDT-SWAP",
        0.095,
        score(68, Direction::Short),
        score(20, Direction::Neutral),
        vec!["dynamic", MULTIDAY_UP_EXTREME],
    );
    symbol.change_15m_pct = -0.035;
    symbol.change_1h_pct = -0.028;
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T06:30:00Z"),
    )
    .is_none());
}

#[test]
fn multiday_up_extension_reversal_window_is_only_small_confirmation_boost() {
    let symbol = symbol(
        "BASED-USDT-SWAP",
        0.136,
        score(73, Direction::Short),
        score(20, Direction::Neutral),
        vec!["dynamic", MULTIDAY_UP_EXTREME],
    );
    let paper = paper_account(10_000.0, Vec::new());
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper,
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper,
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T06:30:00Z"),
    )
    .is_some());
}

#[test]
fn overextended_vwap_distance_blocks_chase_long_even_with_high_score() {
    let mut symbol = symbol(
        "BREV-USDT-SWAP",
        0.0945,
        score(96, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    symbol.scalping_metrics = ScalpingMetrics {
        volume_ratio: 2.4,
        vwap_distance_atr: Some(2.8),
        latest_move_atr: Some(1.2),
        ..ScalpingMetrics::default()
    };
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn pattern_retest_needs_confluence_before_auto_open() {
    let mut symbol = pattern_symbol(99.5, 92, 52, 20, 20, 100.0, 98.0);
    symbol.trend_score = score(20, Direction::Neutral);
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn pattern_entry_uses_trade_score_not_structure_score_only() {
    let symbol = pattern_symbol(99.5, 95, 55, 10, 5, 100.0, 98.0);
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn pattern_entry_rejects_wide_stop_distance() {
    let symbol = pattern_symbol(100.0, 92, 52, 20, 20, 100.0, 90.0);
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn pattern_entry_rejects_poor_reward_risk() {
    let symbol = pattern_symbol(101.9, 92, 52, 20, 20, 100.0, 98.0);
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn pattern_entry_rejects_vwap_overextension() {
    let mut symbol = pattern_symbol(99.5, 92, 52, 20, 20, 100.0, 98.0);
    symbol.scalping_metrics.vwap_distance_atr = Some(2.1);
    assert!(evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .is_none());
}

#[test]
fn pattern_entry_can_open_when_trade_quality_passes() {
    let symbol = pattern_symbol(99.5, 92, 52, 20, 20, 100.0, 98.0);
    let decision = evaluate_auto_strategy_at(
        &symbol,
        &paper_account(10_000.0, Vec::new()),
        AutoStrategyConfig::default(),
        ts_ms("2026-07-02T02:00:00Z"),
    )
    .unwrap();
    match decision {
        AutoStrategyDecision::Open { order, reason, .. } => {
            assert_eq!(order.side, PaperSide::Long);
            assert_eq!(order.primary_signal.as_deref(), Some("pattern_long"));
            assert!(reason.contains("pattern long 92"));
        }
        AutoStrategyDecision::Close { .. } => panic!("expected open decision"),
    }
}

#[test]
fn closes_position_at_stop_loss() {
    let symbol = symbol(
        "LAB-USDT-SWAP",
        9.0,
        score(10, Direction::Neutral),
        score(10, Direction::Neutral),
        vec!["fixed"],
    );
    let paper = paper_account(
        9_700.0,
        vec![position("LAB-USDT-SWAP", PaperSide::Long, -0.31)],
    );
    let decision = evaluate_auto_strategy(&symbol, &paper, AutoStrategyConfig::default()).unwrap();
    match decision {
        AutoStrategyDecision::Close {
            inst_id,
            reason,
            exit_kind,
            execution_price,
            ..
        } => {
            assert_eq!(inst_id, "LAB-USDT-SWAP");
            assert!(reason.contains("stop loss"));
            assert_eq!(exit_kind, AutoExitKind::StopLoss);
            assert_eq!(execution_price, Some(98.5));
        }
        AutoStrategyDecision::Open { .. } => panic!("expected close decision"),
    }
}

#[test]
fn closes_position_at_take_profit() {
    let symbol = symbol(
        "SOL-USDT-SWAP",
        80.0,
        score(10, Direction::Neutral),
        score(10, Direction::Neutral),
        vec!["fixed"],
    );
    let paper = paper_account(
        10_400.0,
        vec![position("SOL-USDT-SWAP", PaperSide::Long, 0.41)],
    );
    let decision = evaluate_auto_strategy(&symbol, &paper, AutoStrategyConfig::default()).unwrap();
    match decision {
        AutoStrategyDecision::Close {
            inst_id,
            reason,
            exit_kind,
            execution_price,
            ..
        } => {
            assert_eq!(inst_id, "SOL-USDT-SWAP");
            assert!(reason.contains("take profit"));
            assert_eq!(exit_kind, AutoExitKind::TakeProfit);
            assert_eq!(execution_price, Some(102.0));
        }
        AutoStrategyDecision::Open { .. } => panic!("expected close decision"),
    }
}

#[test]
fn stop_loss_exit_uses_configured_trigger_price_after_price_gap() {
    let paper = paper_account(
        9_000.0,
        vec![position("TRIA-USDT-SWAP", PaperSide::Short, -2.65)],
    );
    let decision =
        evaluate_auto_exit("TRIA-USDT-SWAP", &paper, AutoStrategyConfig::default()).unwrap();
    match decision {
        AutoStrategyDecision::Close {
            execution_price,
            exit_kind,
            ..
        } => {
            assert_eq!(exit_kind, AutoExitKind::StopLoss);
            assert_close(execution_price.unwrap(), 101.5, 1e-9);
        }
        AutoStrategyDecision::Open { .. } => panic!("expected close decision"),
    }
}

#[test]
fn take_profit_exit_uses_configured_trigger_price_after_price_gap() {
    let paper = paper_account(
        11_000.0,
        vec![position("LAB-USDT-SWAP", PaperSide::Short, 2.57)],
    );
    let decision =
        evaluate_auto_exit("LAB-USDT-SWAP", &paper, AutoStrategyConfig::default()).unwrap();
    match decision {
        AutoStrategyDecision::Close {
            execution_price,
            exit_kind,
            ..
        } => {
            assert_eq!(exit_kind, AutoExitKind::TakeProfit);
            assert_close(execution_price.unwrap(), 98.0, 1e-9);
        }
        AutoStrategyDecision::Open { .. } => panic!("expected close decision"),
    }
}

fn symbol(
    inst_id: &str,
    price: f64,
    trend_score: Score,
    range_score: Score,
    pool_tags: Vec<&str>,
) -> SymbolSnapshot {
    SymbolSnapshot {
        inst_id: inst_id.to_string(),
        price,
        change_5m_pct: 0.0,
        change_15m_pct: 0.0,
        change_1h_pct: 0.0,
        trend_score,
        range_score,
        pool_tags: pool_tags.into_iter().map(String::from).collect(),
        trigger_reason: String::new(),
        funding_rate: None,
        scalping_metrics: ScalpingMetrics::default(),
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

fn pattern_symbol(
    price: f64,
    score_value: u8,
    structure_score: u8,
    confirmation_score: u8,
    hold_score: u8,
    neckline: f64,
    invalidation_level: f64,
) -> SymbolSnapshot {
    let mut symbol = symbol(
        "ETH-USDT-SWAP",
        price,
        score(56, Direction::Long),
        score(20, Direction::Neutral),
        vec!["dynamic"],
    );
    symbol.scalping_metrics = ScalpingMetrics {
        volume_ratio: 2.0,
        vwap_distance_atr: Some(0.2),
        atr_15m_pct: Some(0.01),
        ..ScalpingMetrics::default()
    };
    symbol.pattern_signals = vec![PatternSignal {
        kind: PatternKind::DoubleBottom,
        direction: Direction::Long,
        timeframe: Timeframe::M15,
        status: PatternStatus::Retest,
        score: score_value,
        structure_score,
        confirmation_score,
        hold_score,
        trade_score: structure_score
            .saturating_add(confirmation_score)
            .saturating_add(hold_score),
        neckline: Some(neckline),
        invalidation_level: Some(invalidation_level),
        start_ts_ms: 1,
        confirm_ts_ms: Some(2),
        pivots: Vec::new(),
        level_zone: Some(PatternLevelZone {
            lower: neckline * 0.995,
            upper: neckline * 1.005,
        }),
        reasons: vec!["test pattern".to_string()],
        warnings: Vec::new(),
    }];
    symbol
}

fn ts_ms(raw: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(raw)
        .unwrap()
        .timestamp_millis()
}

fn assert_close(actual: f64, expected: f64, epsilon: f64) {
    assert!(
        (actual - expected).abs() <= epsilon,
        "expected {actual} to be within {epsilon} of {expected}"
    );
}

fn paper_account(equity: f64, positions: Vec<PaperPositionSnapshot>) -> PaperAccountSnapshot {
    let identity = StrategyIdentity::restored_v3();
    let used_margin = positions
        .iter()
        .map(|position| position.margin)
        .sum::<f64>();
    let unrealized_pnl = positions
        .iter()
        .map(|position| position.unrealized_pnl)
        .sum::<f64>();
    PaperAccountSnapshot {
        mode: "paper".to_string(),
        initial_balance: 10_000.0,
        current_strategy_source: SCALPING_OPTIMIZATION_SOURCE.to_string(),
        current_strategy_name: SCALPING_OPTIMIZATION_NAME.to_string(),
        current_strategy_version: SCALPING_OPTIMIZATION_VERSION.to_string(),
        strategy_version: SCALPING_OPTIMIZATION_VERSION.to_string(),
        strategy_build_id: STRATEGY_BUILD_ID.to_string(),
        config_hash: identity.config_hash,
        run_id: INITIAL_RUN_ID.to_string(),
        persistence: PersistenceHealthSnapshot::default(),
        fee_rate: 0.0005,
        slippage_rate: 0.0002,
        total_fees: 0.0,
        total_trades: 0,
        closed_position_count: 0,
        winning_closed_position_count: 0,
        losing_closed_position_count: 0,
        win_rate: None,
        average_holding_duration_ms: None,
        average_closed_position_pnl: None,
        average_winning_pnl: None,
        average_losing_pnl: None,
        profit_factor: None,
        largest_winning_pnl: None,
        largest_losing_pnl: None,
        strategy_stats: Vec::new(),
        realized_pnl: equity - 10_000.0 - unrealized_pnl,
        unrealized_pnl,
        equity,
        used_margin,
        available_balance: equity - used_margin,
        equity_history: Vec::new(),
        positions,
        position_history: Vec::new(),
        trades: Vec::new(),
    }
}

fn position(inst_id: &str, side: PaperSide, pnl_pct: f64) -> PaperPositionSnapshot {
    let margin = 300.0;
    PaperPositionSnapshot {
        inst_id: inst_id.to_string(),
        side,
        qty: 1.0,
        entry_price: 100.0,
        mark_price: 100.0,
        margin,
        leverage: 20.0,
        notional: margin * 20.0,
        unrealized_pnl: margin * pnl_pct,
        pnl_pct,
        opened_at_ms: 1,
        source: "manual".to_string(),
        strategy_name: "manual".to_string(),
        strategy_version: "manual".to_string(),
        primary_signal: String::new(),
        reason: "manual".to_string(),
        fee: 0.0,
        config_hash: StrategyIdentity::restored_v3().config_hash,
        signal_tags: Vec::new(),
        tags: Vec::new(),
        stop_loss: None,
        take_profit: None,
        expire_at_ms: None,
    }
}
