use alphapulse_okx_backend::risk_safety::{
    AccountAction, AccountEvent, AccountEventEnvelope, AccountEventResult, AccountRiskState,
    AccountScope, RiskCommand, RiskMode,
};

fn envelope(id: &str, stream: &str, sequence: u64, event: AccountEvent) -> AccountEventEnvelope {
    AccountEventEnvelope {
        event_id: id.to_string(),
        stream: stream.to_string(),
        sequence,
        event,
    }
}

#[test]
fn delayed_market_data_enters_close_only_without_blocking_exits() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    let result = risk.handle(envelope(
        "ticker-1",
        "okx:ticker:BTC-USDT-SWAP",
        1,
        AccountEvent::MarketData {
            event_ts_ms: 1_000,
            received_at_ms: 20_000,
            max_lag_ms: 5_000,
        },
    ));

    assert!(matches!(result, AccountEventResult::Applied { .. }));
    assert_eq!(risk.snapshot().mode, RiskMode::CloseOnly);
    assert!(!risk.allows(AccountAction::Open));
    assert!(risk.allows(AccountAction::Close));
    assert!(risk.allows(AccountAction::StopLoss));
}

#[test]
fn stop_gap_requests_market_close_and_records_actual_slippage() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    let result = risk.handle(envelope(
        "stop-1",
        "okx:orders",
        1,
        AccountEvent::StopTriggered {
            symbol: "BTC-USDT-SWAP".to_string(),
            trigger_price: 100.0,
            market_price: 92.0,
        },
    ));

    let AccountEventResult::Applied { commands } = result else {
        panic!("stop event should be applied");
    };
    assert_eq!(
        commands,
        vec![RiskCommand::EmergencyMarketClose {
            symbol: "BTC-USDT-SWAP".to_string(),
            reason: "stop_triggered".to_string(),
            trigger_price: Some(100.0),
            observed_market_price: Some(92.0),
            actual_slippage_rate: Some(0.08),
        }]
    );
}

#[test]
fn restart_with_positions_requires_reconciliation_before_entries_resume() {
    let mut risk = AccountRiskState::startup(AccountScope::default(), true);
    assert_eq!(risk.snapshot().mode, RiskMode::Reconciling);
    assert!(!risk.allows(AccountAction::Open));

    risk.handle(envelope(
        "ws-up",
        "system:websocket",
        1,
        AccountEvent::WebsocketConnection { connected: true },
    ));

    risk.handle(envelope(
        "reconcile-1",
        "rest:account",
        1,
        AccountEvent::RestReconciled {
            positions_match: true,
        },
    ));
    assert_eq!(risk.snapshot().mode, RiskMode::Normal);
    assert!(risk.allows(AccountAction::Open));
}

#[test]
fn duplicate_callbacks_are_idempotent_and_old_sequences_are_rejected() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    let first = envelope("fill-7", "okx:orders", 7, AccountEvent::EquityUpdated);
    assert!(matches!(
        risk.handle(first.clone()),
        AccountEventResult::Applied { .. }
    ));
    assert_eq!(risk.handle(first), AccountEventResult::Duplicate);
    assert_eq!(
        risk.handle(envelope(
            "fill-6",
            "okx:orders",
            6,
            AccountEvent::EquityUpdated,
        )),
        AccountEventResult::Stale { last_sequence: 7 }
    );
}

#[test]
fn websocket_disconnect_blocks_entries_until_rest_reconciliation() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    risk.handle(envelope(
        "ws-down",
        "system:websocket",
        1,
        AccountEvent::WebsocketConnection { connected: false },
    ));
    assert_eq!(risk.snapshot().mode, RiskMode::CloseOnly);

    risk.handle(envelope(
        "ws-up",
        "system:websocket",
        2,
        AccountEvent::WebsocketConnection { connected: true },
    ));
    assert_eq!(risk.snapshot().mode, RiskMode::Reconciling);

    risk.handle(envelope(
        "rest-ok",
        "rest:account",
        1,
        AccountEvent::RestReconciled {
            positions_match: true,
        },
    ));
    assert_eq!(risk.snapshot().mode, RiskMode::Normal);
}

#[test]
fn rest_reconciliation_does_not_enable_entries_while_websocket_is_still_down() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    risk.handle(envelope(
        "ws-down",
        "system:websocket",
        1,
        AccountEvent::WebsocketConnection { connected: false },
    ));
    risk.handle(envelope(
        "rest-ok",
        "rest:account",
        1,
        AccountEvent::RestReconciled {
            positions_match: true,
        },
    ));

    assert_eq!(risk.snapshot().mode, RiskMode::CloseOnly);
    assert!(!risk.allows(AccountAction::Open));
}

#[test]
fn redis_failure_is_observable_but_does_not_gate_risk_execution() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    risk.handle(envelope(
        "redis-down",
        "system:redis",
        1,
        AccountEvent::RedisHealth { available: false },
    ));

    assert!(!risk.snapshot().redis_available);
    assert_eq!(risk.snapshot().mode, RiskMode::Normal);
    assert!(risk.allows(AccountAction::Open));
    assert!(risk.allows(AccountAction::StopLoss));
}

#[test]
fn rejected_stop_escalates_to_emergency_market_close() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    let result = risk.handle(envelope(
        "stop-rejected-1",
        "okx:orders",
        1,
        AccountEvent::StopOrderRejected {
            symbol: "ETH-USDT-SWAP".to_string(),
            observed_market_price: Some(3_000.0),
        },
    ));

    let AccountEventResult::Applied { commands } = result else {
        panic!("rejection should be applied");
    };
    assert!(matches!(
        commands.as_slice(),
        [RiskCommand::EmergencyMarketClose { reason, .. }] if reason == "stop_order_rejected"
    ));
}

#[test]
fn partial_fill_immediately_protects_filled_quantity() {
    let mut risk = AccountRiskState::ready(AccountScope::default());
    let result = risk.handle(envelope(
        "partial-1",
        "okx:orders",
        1,
        AccountEvent::PartialFill {
            symbol: "SOL-USDT-SWAP".to_string(),
            filled_quantity: 2.5,
            protection_price: 140.0,
        },
    ));

    let AccountEventResult::Applied { commands } = result else {
        panic!("partial fill should be applied");
    };
    assert_eq!(
        commands,
        vec![RiskCommand::ProtectFilledQuantity {
            symbol: "SOL-USDT-SWAP".to_string(),
            quantity: 2.5,
            stop_price: 140.0,
        }]
    );
}

#[test]
fn redis_keys_are_isolated_by_tenant_and_account() {
    let left = AccountScope::new("tenant-a", "paper-1").unwrap();
    let right = AccountScope::new("tenant-b", "paper-1").unwrap();
    assert_eq!(
        left.redis_key("dashboard:snapshot"),
        "alphapulse:tenant-a:paper-1:dashboard:snapshot"
    );
    assert_ne!(
        left.redis_key("dashboard:snapshot"),
        right.redis_key("dashboard:snapshot")
    );
    assert!(AccountScope::new("tenant:a", "paper-1").is_err());
}
