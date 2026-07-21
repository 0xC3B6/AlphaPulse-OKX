use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

pub const DEFAULT_TENANT_ID: &str = "default";
pub const DEFAULT_ACCOUNT_ID: &str = "paper";
const MAX_PROCESSED_EVENT_IDS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountScope {
    pub tenant_id: String,
    pub account_id: String,
}

impl Default for AccountScope {
    fn default() -> Self {
        Self {
            tenant_id: DEFAULT_TENANT_ID.to_string(),
            account_id: DEFAULT_ACCOUNT_ID.to_string(),
        }
    }
}

impl AccountScope {
    pub fn new(
        tenant_id: impl Into<String>,
        account_id: impl Into<String>,
    ) -> Result<Self, AccountScopeError> {
        let scope = Self {
            tenant_id: tenant_id.into(),
            account_id: account_id.into(),
        };
        validate_scope_segment("tenant_id", &scope.tenant_id)?;
        validate_scope_segment("account_id", &scope.account_id)?;
        Ok(scope)
    }

    pub fn redis_key(&self, suffix: &str) -> String {
        format!(
            "alphapulse:{}:{}:{}",
            self.tenant_id, self.account_id, suffix
        )
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AccountScopeError {
    #[error("{0} must contain only ASCII letters, digits, '-' or '_'")]
    InvalidSegment(&'static str),
}

fn validate_scope_segment(name: &'static str, value: &str) -> Result<(), AccountScopeError> {
    let valid = !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    valid
        .then_some(())
        .ok_or(AccountScopeError::InvalidSegment(name))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskMode {
    Normal,
    CloseOnly,
    Reconciling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountAction {
    Open,
    Close,
    Fee,
    StopLoss,
    AccountKillSwitch,
    EquityUpdate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccountRiskSnapshot {
    pub scope: AccountScope,
    pub mode: RiskMode,
    pub reasons: Vec<String>,
    pub websocket_connected: bool,
    pub redis_available: bool,
    pub reconciliation_required: bool,
    pub last_market_event_at_ms: Option<i64>,
    pub last_sequences: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountEventEnvelope {
    pub event_id: String,
    pub stream: String,
    pub sequence: u64,
    pub event: AccountEvent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccountEvent {
    ServiceStarted {
        has_positions: bool,
    },
    MarketData {
        event_ts_ms: i64,
        received_at_ms: i64,
        max_lag_ms: i64,
    },
    WebsocketConnection {
        connected: bool,
    },
    RestReconciled {
        positions_match: bool,
    },
    RedisHealth {
        available: bool,
    },
    StopTriggered {
        symbol: String,
        trigger_price: f64,
        market_price: f64,
    },
    StopOrderRejected {
        symbol: String,
        observed_market_price: Option<f64>,
    },
    PartialFill {
        symbol: String,
        filled_quantity: f64,
        protection_price: f64,
    },
    AccountKillSwitch {
        active: bool,
    },
    EquityUpdated,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RiskCommand {
    EmergencyMarketClose {
        symbol: String,
        reason: String,
        trigger_price: Option<f64>,
        observed_market_price: Option<f64>,
        actual_slippage_rate: Option<f64>,
    },
    ProtectFilledQuantity {
        symbol: String,
        quantity: f64,
        stop_price: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccountEventResult {
    Applied { commands: Vec<RiskCommand> },
    Duplicate,
    Stale { last_sequence: u64 },
}

#[derive(Debug, Clone)]
pub struct AccountRiskState {
    scope: AccountScope,
    reasons: BTreeSet<String>,
    websocket_connected: bool,
    redis_available: bool,
    last_market_event_at_ms: Option<i64>,
    last_sequences: BTreeMap<String, u64>,
    processed_event_ids: BTreeSet<String>,
    processed_event_order: VecDeque<String>,
}

impl AccountRiskState {
    pub fn ready(scope: AccountScope) -> Self {
        Self {
            scope,
            reasons: BTreeSet::new(),
            websocket_connected: true,
            redis_available: true,
            last_market_event_at_ms: None,
            last_sequences: BTreeMap::new(),
            processed_event_ids: BTreeSet::new(),
            processed_event_order: VecDeque::new(),
        }
    }

    pub fn startup(scope: AccountScope, has_positions: bool) -> Self {
        let mut state = Self::ready(scope);
        state.websocket_connected = false;
        state.reasons.insert("websocket_disconnected".to_string());
        if has_positions {
            state.reasons.insert("restart_reconciliation".to_string());
        }
        state
    }

    pub fn snapshot(&self) -> AccountRiskSnapshot {
        AccountRiskSnapshot {
            scope: self.scope.clone(),
            mode: self.mode(),
            reasons: self.reasons.iter().cloned().collect(),
            websocket_connected: self.websocket_connected,
            redis_available: self.redis_available,
            reconciliation_required: self.reconciliation_required(),
            last_market_event_at_ms: self.last_market_event_at_ms,
            last_sequences: self.last_sequences.clone(),
        }
    }

    pub fn allows(&self, action: AccountAction) -> bool {
        action != AccountAction::Open || self.mode() == RiskMode::Normal
    }

    pub fn entry_block_reason(&self) -> Option<String> {
        (!self.allows(AccountAction::Open)).then(|| {
            if self.reasons.is_empty() {
                "account is close-only".to_string()
            } else {
                self.reasons.iter().cloned().collect::<Vec<_>>().join(",")
            }
        })
    }

    pub fn handle(&mut self, envelope: AccountEventEnvelope) -> AccountEventResult {
        if self.processed_event_ids.contains(&envelope.event_id) {
            return AccountEventResult::Duplicate;
        }
        if let Some(last_sequence) = self.last_sequences.get(&envelope.stream).copied() {
            if envelope.sequence <= last_sequence {
                return AccountEventResult::Stale { last_sequence };
            }
        }

        self.last_sequences
            .insert(envelope.stream, envelope.sequence);
        self.remember_event(envelope.event_id);
        let mut commands = Vec::new();
        match envelope.event {
            AccountEvent::ServiceStarted { has_positions } => {
                if has_positions {
                    self.reasons.insert("restart_reconciliation".to_string());
                }
            }
            AccountEvent::MarketData {
                event_ts_ms,
                received_at_ms,
                max_lag_ms,
            } => {
                self.last_market_event_at_ms = Some(event_ts_ms);
                if received_at_ms.saturating_sub(event_ts_ms) > max_lag_ms.max(0) {
                    self.reasons.insert("market_data_stale".to_string());
                } else {
                    self.reasons.remove("market_data_stale");
                }
            }
            AccountEvent::WebsocketConnection { connected } => {
                self.websocket_connected = connected;
                if connected {
                    self.reasons.remove("websocket_disconnected");
                } else {
                    self.reasons.insert("websocket_disconnected".to_string());
                    self.reasons
                        .insert("rest_reconciliation_required".to_string());
                }
            }
            AccountEvent::RestReconciled { positions_match } => {
                if positions_match {
                    self.reasons.remove("restart_reconciliation");
                    self.reasons.remove("rest_reconciliation_required");
                    self.reasons.remove("reconciliation_mismatch");
                    self.reasons.remove("market_data_stale");
                } else {
                    self.reasons.insert("reconciliation_mismatch".to_string());
                }
            }
            AccountEvent::RedisHealth { available } => {
                self.redis_available = available;
            }
            AccountEvent::StopTriggered {
                symbol,
                trigger_price,
                market_price,
            } => {
                commands.push(RiskCommand::EmergencyMarketClose {
                    symbol,
                    reason: "stop_triggered".to_string(),
                    trigger_price: valid_positive(trigger_price).then_some(trigger_price),
                    observed_market_price: valid_positive(market_price).then_some(market_price),
                    actual_slippage_rate: actual_slippage_rate(trigger_price, market_price),
                });
            }
            AccountEvent::StopOrderRejected {
                symbol,
                observed_market_price,
            } => {
                commands.push(RiskCommand::EmergencyMarketClose {
                    symbol,
                    reason: "stop_order_rejected".to_string(),
                    trigger_price: None,
                    observed_market_price,
                    actual_slippage_rate: None,
                });
            }
            AccountEvent::PartialFill {
                symbol,
                filled_quantity,
                protection_price,
            } => {
                if valid_positive(filled_quantity) && valid_positive(protection_price) {
                    commands.push(RiskCommand::ProtectFilledQuantity {
                        symbol,
                        quantity: filled_quantity,
                        stop_price: protection_price,
                    });
                }
            }
            AccountEvent::AccountKillSwitch { active } => {
                if active {
                    self.reasons.insert("account_kill_switch".to_string());
                } else {
                    self.reasons.remove("account_kill_switch");
                }
            }
            AccountEvent::EquityUpdated => {}
        }
        AccountEventResult::Applied { commands }
    }

    fn remember_event(&mut self, event_id: String) {
        self.processed_event_ids.insert(event_id.clone());
        self.processed_event_order.push_back(event_id);
        while self.processed_event_order.len() > MAX_PROCESSED_EVENT_IDS {
            if let Some(expired) = self.processed_event_order.pop_front() {
                self.processed_event_ids.remove(&expired);
            }
        }
    }

    fn reconciliation_required(&self) -> bool {
        self.reasons.contains("restart_reconciliation")
            || self.reasons.contains("rest_reconciliation_required")
            || self.reasons.contains("reconciliation_mismatch")
    }

    fn mode(&self) -> RiskMode {
        if self.reasons.contains("restart_reconciliation")
            || self.reasons.contains("reconciliation_mismatch")
            || (self.reasons.contains("rest_reconciliation_required")
                && !self.reasons.contains("websocket_disconnected"))
        {
            RiskMode::Reconciling
        } else if self.reasons.is_empty() {
            RiskMode::Normal
        } else {
            RiskMode::CloseOnly
        }
    }
}

fn actual_slippage_rate(trigger_price: f64, market_price: f64) -> Option<f64> {
    if !valid_positive(trigger_price) || !valid_positive(market_price) {
        return None;
    }
    Some(((trigger_price - market_price).abs() / trigger_price).max(0.0))
}

fn valid_positive(value: f64) -> bool {
    value > 0.0 && value.is_finite()
}
