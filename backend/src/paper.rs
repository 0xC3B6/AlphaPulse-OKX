use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::domain::SymbolSnapshot;

const DEFAULT_INITIAL_BALANCE: f64 = 10_000.0;
const MAX_LEVERAGE: f64 = 50.0;
const MAX_TRADES: usize = 100;
const MAX_POSITION_HISTORY: usize = 500;
const DEFAULT_VERSION_CODE: &str = "v0.1.3";
const DEFAULT_RUN_ID: &str = "v0.1.3-paper-1";
const DEFAULT_STRATEGY_NAME: &str = "Scalping Optimization Design";
const DEFAULT_SOURCE: &str = "scalping_optimization_design";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperSide {
    Long,
    Short,
}

impl PaperSide {
    fn pnl(self, entry_price: f64, mark_price: f64, qty: f64) -> f64 {
        match self {
            Self::Long => (mark_price - entry_price) * qty,
            Self::Short => (entry_price - mark_price) * qty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaperTradeAction {
    Open,
    Close,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaperOrderRequest {
    pub inst_id: String,
    pub side: PaperSide,
    pub margin: f64,
    pub leverage: f64,
    #[serde(default)]
    pub version_code: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub primary_signal: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub risk_flags: Vec<String>,
    #[serde(default)]
    pub risk_guard_decision: Option<String>,
    #[serde(default)]
    pub strategy_reason: Option<String>,
    #[serde(default)]
    pub config_hash: Option<String>,
}

impl Default for PaperOrderRequest {
    fn default() -> Self {
        Self {
            inst_id: String::new(),
            side: PaperSide::Long,
            margin: 0.0,
            leverage: 1.0,
            version_code: None,
            run_id: None,
            primary_signal: None,
            tags: Vec::new(),
            risk_flags: Vec::new(),
            risk_guard_decision: None,
            strategy_reason: None,
            config_hash: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperAccountSnapshot {
    pub mode: &'static str,
    pub initial_balance: f64,
    pub fee_rate: f64,
    pub slippage_rate: f64,
    pub total_fees: f64,
    pub total_trades: usize,
    pub closed_position_count: usize,
    pub winning_closed_position_count: usize,
    pub losing_closed_position_count: usize,
    pub win_rate: Option<f64>,
    pub average_holding_duration_ms: Option<f64>,
    pub average_closed_position_pnl: Option<f64>,
    pub average_winning_pnl: Option<f64>,
    pub average_losing_pnl: Option<f64>,
    pub profit_factor: Option<f64>,
    pub largest_winning_pnl: Option<f64>,
    pub largest_losing_pnl: Option<f64>,
    pub strategy_stats: Vec<PaperStrategyStats>,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub equity: f64,
    pub used_margin: f64,
    pub available_balance: f64,
    pub positions: Vec<PaperPositionSnapshot>,
    pub position_history: Vec<PaperClosedPositionSnapshot>,
    pub trades: Vec<PaperTrade>,
}

impl Default for PaperAccountSnapshot {
    fn default() -> Self {
        Self {
            mode: "paper",
            initial_balance: DEFAULT_INITIAL_BALANCE,
            fee_rate: 0.0,
            slippage_rate: 0.0,
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
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            equity: DEFAULT_INITIAL_BALANCE,
            used_margin: 0.0,
            available_balance: DEFAULT_INITIAL_BALANCE,
            positions: Vec::new(),
            position_history: Vec::new(),
            trades: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperPositionSnapshot {
    pub inst_id: String,
    pub side: PaperSide,
    pub qty: f64,
    pub entry_price: f64,
    pub mark_price: f64,
    pub margin: f64,
    pub leverage: f64,
    pub notional: f64,
    pub unrealized_pnl: f64,
    pub pnl_pct: f64,
    pub opened_at_ms: i64,
    pub source: String,
    pub strategy_name: String,
    pub strategy_version: String,
    pub version_code: String,
    pub run_id: String,
    pub primary_signal: String,
    pub reason: String,
    pub tags: Vec<String>,
    pub risk_flags: Vec<String>,
    pub risk_guard_decision: Option<String>,
    pub strategy_reason: String,
    pub config_hash: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperTrade {
    pub id: u64,
    pub inst_id: String,
    pub side: PaperSide,
    pub action: PaperTradeAction,
    pub price: f64,
    pub qty: f64,
    pub margin: f64,
    pub notional: f64,
    pub fee: f64,
    pub slippage_rate: f64,
    pub source: String,
    pub strategy_name: String,
    pub strategy_version: String,
    pub version_code: String,
    pub run_id: String,
    pub primary_signal: String,
    pub reason: String,
    pub tags: Vec<String>,
    pub risk_flags: Vec<String>,
    pub risk_guard_decision: Option<String>,
    pub strategy_reason: String,
    pub config_hash: String,
    pub realized_pnl: f64,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperClosedPositionSnapshot {
    pub id: u64,
    pub inst_id: String,
    pub side: PaperSide,
    pub qty: f64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub margin: f64,
    pub leverage: f64,
    pub notional: f64,
    pub fees: f64,
    pub realized_pnl: f64,
    pub pnl_pct: f64,
    pub opened_at_ms: i64,
    pub closed_at_ms: i64,
    pub duration_ms: i64,
    pub source: String,
    pub strategy_name: String,
    pub strategy_version: String,
    pub version_code: String,
    pub run_id: String,
    pub primary_signal: String,
    pub reason: String,
    pub close_source: String,
    pub close_reason: String,
    pub tags: Vec<String>,
    pub open_tags: Vec<String>,
    pub close_tags: Vec<String>,
    pub risk_flags: Vec<String>,
    pub risk_guard_decision: Option<String>,
    pub strategy_reason: String,
    pub config_hash: String,
    pub max_adverse_excursion: Option<f64>,
    pub max_favorable_excursion: Option<f64>,
    pub planned_risk_usdt: Option<f64>,
    pub r_multiple: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperStrategyStats {
    pub strategy_name: String,
    pub strategy_version: String,
    pub total_trades: usize,
    pub closed_position_count: usize,
    pub winning_closed_position_count: usize,
    pub losing_closed_position_count: usize,
    pub win_rate: Option<f64>,
    pub realized_pnl: f64,
    pub total_fees: f64,
    pub first_trade_ts_ms: Option<i64>,
    pub last_trade_ts_ms: Option<i64>,
    pub running_duration_ms: Option<i64>,
    pub average_holding_duration_ms: Option<f64>,
    pub average_position_pnl: Option<f64>,
    pub profit_factor: Option<f64>,
    pub largest_winning_pnl: Option<f64>,
    pub largest_losing_pnl: Option<f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum PaperError {
    #[error("instrument is required")]
    EmptyInstrument,
    #[error("price unavailable for {0}")]
    PriceUnavailable(String),
    #[error("margin must be greater than zero")]
    InvalidMargin,
    #[error("leverage must be between 1x and 50x")]
    InvalidLeverage,
    #[error("insufficient simulated balance")]
    InsufficientBalance,
    #[error("opposite simulated position exists; close it first")]
    OppositePosition,
    #[error("no simulated position for {0}")]
    PositionNotFound(String),
}

#[derive(Debug, Clone)]
pub struct PaperState {
    initial_balance: f64,
    realized_pnl: f64,
    positions: BTreeMap<String, PaperPosition>,
    position_history: VecDeque<PaperClosedPositionSnapshot>,
    trades: VecDeque<PaperTrade>,
    next_trade_id: u64,
    next_closed_position_id: u64,
    fee_rate: f64,
    slippage_rate: f64,
    total_fees: f64,
}

#[derive(Debug, Clone)]
struct PaperPosition {
    inst_id: String,
    side: PaperSide,
    qty: f64,
    entry_price: f64,
    margin: f64,
    leverage: f64,
    notional: f64,
    opened_at_ms: i64,
    metadata: PaperMetadata,
}

#[derive(Debug, Clone)]
struct PaperMetadata {
    source: String,
    strategy_name: String,
    strategy_version: String,
    version_code: String,
    run_id: String,
    primary_signal: String,
    reason: String,
    tags: Vec<String>,
    risk_flags: Vec<String>,
    risk_guard_decision: Option<String>,
    strategy_reason: String,
    config_hash: String,
}

impl Default for PaperState {
    fn default() -> Self {
        Self {
            initial_balance: DEFAULT_INITIAL_BALANCE,
            realized_pnl: 0.0,
            positions: BTreeMap::new(),
            position_history: VecDeque::new(),
            trades: VecDeque::new(),
            next_trade_id: 1,
            next_closed_position_id: 1,
            fee_rate: 0.0,
            slippage_rate: 0.0,
            total_fees: 0.0,
        }
    }
}

impl PaperState {
    pub fn snapshot(&self, prices: &BTreeMap<String, SymbolSnapshot>) -> PaperAccountSnapshot {
        let positions: Vec<_> = self
            .positions
            .values()
            .map(|position| self.position_snapshot(position, prices))
            .collect();
        let position_history: Vec<_> = self.position_history.iter().rev().cloned().collect();
        let used_margin = positions
            .iter()
            .map(|position| position.margin)
            .sum::<f64>();
        let unrealized_pnl = positions
            .iter()
            .map(|position| position.unrealized_pnl)
            .sum::<f64>();
        let equity = self.initial_balance + self.realized_pnl + unrealized_pnl;
        let available_balance = equity - used_margin;
        let winning_closed_position_count = self
            .position_history
            .iter()
            .filter(|position| position.realized_pnl > 0.0)
            .count();
        let losing_closed_position_count = self
            .position_history
            .iter()
            .filter(|position| position.realized_pnl < 0.0)
            .count();
        let closed_position_count = self.position_history.len();
        let winners = self
            .position_history
            .iter()
            .filter(|position| position.realized_pnl > 0.0)
            .cloned()
            .collect::<Vec<_>>();
        let losers = self
            .position_history
            .iter()
            .filter(|position| position.realized_pnl < 0.0)
            .cloned()
            .collect::<Vec<_>>();
        let gross_profit = winners
            .iter()
            .map(|position| position.realized_pnl)
            .sum::<f64>();
        let gross_loss_abs = losers
            .iter()
            .map(|position| position.realized_pnl)
            .sum::<f64>()
            .abs();

        PaperAccountSnapshot {
            mode: "paper",
            initial_balance: self.initial_balance,
            fee_rate: self.fee_rate,
            slippage_rate: self.slippage_rate,
            total_fees: self.total_fees,
            total_trades: self.trades.len(),
            closed_position_count,
            winning_closed_position_count,
            losing_closed_position_count,
            win_rate: ratio(winning_closed_position_count, closed_position_count),
            average_holding_duration_ms: average(
                self.position_history
                    .iter()
                    .map(|position| position.duration_ms as f64),
            ),
            average_closed_position_pnl: average(
                self.position_history
                    .iter()
                    .map(|position| position.realized_pnl),
            ),
            average_winning_pnl: average(winners.iter().map(|position| position.realized_pnl)),
            average_losing_pnl: average(losers.iter().map(|position| position.realized_pnl)),
            profit_factor: if gross_profit > 0.0 && gross_loss_abs > 0.0 {
                Some(gross_profit / gross_loss_abs)
            } else {
                None
            },
            largest_winning_pnl: max_number(winners.iter().map(|position| position.realized_pnl)),
            largest_losing_pnl: min_number(losers.iter().map(|position| position.realized_pnl)),
            strategy_stats: build_strategy_stats(&self.position_history),
            realized_pnl: self.realized_pnl,
            unrealized_pnl,
            equity,
            used_margin,
            available_balance,
            positions,
            position_history,
            trades: self.trades.iter().rev().cloned().collect(),
        }
    }

    pub fn has_open_positions(&self) -> bool {
        !self.positions.is_empty()
    }

    pub fn has_position(&self, inst_id: &str) -> bool {
        self.positions.contains_key(inst_id)
    }

    pub fn position_pnl_pct(
        &self,
        inst_id: &str,
        side: PaperSide,
        prices: &BTreeMap<String, SymbolSnapshot>,
    ) -> Option<f64> {
        let position = self.positions.get(inst_id)?;
        if position.side != side {
            return None;
        }
        Some(self.position_snapshot(position, prices).pnl_pct)
    }

    pub fn open(
        &mut self,
        request: PaperOrderRequest,
        price: f64,
        available_balance: f64,
        ts_ms: i64,
    ) -> Result<PaperTrade, PaperError> {
        validate_order(&request, price)?;
        if available_balance < request.margin {
            return Err(PaperError::InsufficientBalance);
        }

        let metadata = PaperMetadata::from_request(&request);
        let qty = request.margin * request.leverage / price;
        let notional = qty * price;

        if let Some(existing) = self.positions.get_mut(&request.inst_id) {
            if existing.side != request.side {
                return Err(PaperError::OppositePosition);
            }
            existing.qty += qty;
            existing.margin += request.margin;
            existing.notional += notional;
            existing.entry_price = existing.notional / existing.qty;
            existing.leverage = existing.notional / existing.margin;
        } else {
            self.positions.insert(
                request.inst_id.clone(),
                PaperPosition {
                    inst_id: request.inst_id.clone(),
                    side: request.side,
                    qty,
                    entry_price: price,
                    margin: request.margin,
                    leverage: request.leverage,
                    notional,
                    opened_at_ms: ts_ms,
                    metadata: metadata.clone(),
                },
            );
        }

        let trade = PaperTrade {
            id: self.next_trade_id(),
            inst_id: request.inst_id,
            side: request.side,
            action: PaperTradeAction::Open,
            price,
            qty,
            margin: request.margin,
            notional,
            fee: 0.0,
            slippage_rate: self.slippage_rate,
            source: metadata.source.clone(),
            strategy_name: metadata.strategy_name.clone(),
            strategy_version: metadata.strategy_version.clone(),
            version_code: metadata.version_code.clone(),
            run_id: metadata.run_id.clone(),
            primary_signal: metadata.primary_signal.clone(),
            reason: metadata.reason.clone(),
            tags: metadata.tags.clone(),
            risk_flags: metadata.risk_flags.clone(),
            risk_guard_decision: metadata.risk_guard_decision.clone(),
            strategy_reason: metadata.strategy_reason.clone(),
            config_hash: metadata.config_hash.clone(),
            realized_pnl: 0.0,
            ts_ms,
        };
        self.push_trade(trade.clone());
        Ok(trade)
    }

    pub fn close(
        &mut self,
        inst_id: &str,
        price: f64,
        ts_ms: i64,
    ) -> Result<PaperTrade, PaperError> {
        if price <= 0.0 || !price.is_finite() {
            return Err(PaperError::PriceUnavailable(inst_id.to_string()));
        }
        let position = self
            .positions
            .remove(inst_id)
            .ok_or_else(|| PaperError::PositionNotFound(inst_id.to_string()))?;
        let realized_pnl = position.side.pnl(position.entry_price, price, position.qty);
        self.realized_pnl += realized_pnl;
        let close_reason = if realized_pnl < 0.0 {
            format!(
                "{} stop_loss {:.2}",
                position.metadata.strategy_version, realized_pnl
            )
        } else {
            format!(
                "{} take_profit {:.2}",
                position.metadata.strategy_version, realized_pnl
            )
        };

        let trade = PaperTrade {
            id: self.next_trade_id(),
            inst_id: position.inst_id,
            side: position.side,
            action: PaperTradeAction::Close,
            price,
            qty: position.qty,
            margin: position.margin,
            notional: position.qty * price,
            fee: 0.0,
            slippage_rate: self.slippage_rate,
            source: position.metadata.source.clone(),
            strategy_name: position.metadata.strategy_name.clone(),
            strategy_version: position.metadata.strategy_version.clone(),
            version_code: position.metadata.version_code.clone(),
            run_id: position.metadata.run_id.clone(),
            primary_signal: position.metadata.primary_signal.clone(),
            reason: close_reason.clone(),
            tags: position.metadata.tags.clone(),
            risk_flags: position.metadata.risk_flags.clone(),
            risk_guard_decision: position.metadata.risk_guard_decision.clone(),
            strategy_reason: position.metadata.strategy_reason.clone(),
            config_hash: position.metadata.config_hash.clone(),
            realized_pnl,
            ts_ms,
        };
        let closed_position = PaperClosedPositionSnapshot {
            id: self.next_closed_position_id(),
            inst_id: trade.inst_id.clone(),
            side: trade.side,
            qty: trade.qty,
            entry_price: position.entry_price,
            exit_price: price,
            margin: position.margin,
            leverage: position.leverage,
            notional: trade.notional,
            fees: 0.0,
            realized_pnl,
            pnl_pct: if position.margin > 0.0 {
                realized_pnl / position.margin
            } else {
                0.0
            },
            opened_at_ms: position.opened_at_ms,
            closed_at_ms: ts_ms,
            duration_ms: ts_ms.saturating_sub(position.opened_at_ms),
            source: position.metadata.source.clone(),
            strategy_name: position.metadata.strategy_name.clone(),
            strategy_version: position.metadata.strategy_version.clone(),
            version_code: position.metadata.version_code.clone(),
            run_id: position.metadata.run_id.clone(),
            primary_signal: position.metadata.primary_signal.clone(),
            reason: position.metadata.reason.clone(),
            close_source: position.metadata.source.clone(),
            close_reason,
            tags: position.metadata.tags.clone(),
            open_tags: position.metadata.tags.clone(),
            close_tags: Vec::new(),
            risk_flags: position.metadata.risk_flags.clone(),
            risk_guard_decision: position.metadata.risk_guard_decision.clone(),
            strategy_reason: position.metadata.strategy_reason.clone(),
            config_hash: position.metadata.config_hash.clone(),
            max_adverse_excursion: None,
            max_favorable_excursion: None,
            planned_risk_usdt: None,
            r_multiple: None,
        };
        self.push_closed_position(closed_position);
        self.push_trade(trade.clone());
        Ok(trade)
    }

    fn position_snapshot(
        &self,
        position: &PaperPosition,
        prices: &BTreeMap<String, SymbolSnapshot>,
    ) -> PaperPositionSnapshot {
        let mark_price = prices
            .get(&position.inst_id)
            .map(|symbol| symbol.price)
            .filter(|price| *price > 0.0 && price.is_finite())
            .unwrap_or(position.entry_price);
        let unrealized_pnl = position
            .side
            .pnl(position.entry_price, mark_price, position.qty);
        let pnl_pct = if position.margin > 0.0 {
            unrealized_pnl / position.margin
        } else {
            0.0
        };

        PaperPositionSnapshot {
            inst_id: position.inst_id.clone(),
            side: position.side,
            qty: position.qty,
            entry_price: position.entry_price,
            mark_price,
            margin: position.margin,
            leverage: position.leverage,
            notional: position.notional,
            unrealized_pnl,
            pnl_pct,
            opened_at_ms: position.opened_at_ms,
            source: position.metadata.source.clone(),
            strategy_name: position.metadata.strategy_name.clone(),
            strategy_version: position.metadata.strategy_version.clone(),
            version_code: position.metadata.version_code.clone(),
            run_id: position.metadata.run_id.clone(),
            primary_signal: position.metadata.primary_signal.clone(),
            reason: position.metadata.reason.clone(),
            tags: position.metadata.tags.clone(),
            risk_flags: position.metadata.risk_flags.clone(),
            risk_guard_decision: position.metadata.risk_guard_decision.clone(),
            strategy_reason: position.metadata.strategy_reason.clone(),
            config_hash: position.metadata.config_hash.clone(),
        }
    }

    fn next_trade_id(&mut self) -> u64 {
        let id = self.next_trade_id;
        self.next_trade_id += 1;
        id
    }

    fn push_trade(&mut self, trade: PaperTrade) {
        self.trades.push_back(trade);
        while self.trades.len() > MAX_TRADES {
            self.trades.pop_front();
        }
    }

    fn next_closed_position_id(&mut self) -> u64 {
        let id = self.next_closed_position_id;
        self.next_closed_position_id += 1;
        id
    }

    fn push_closed_position(&mut self, position: PaperClosedPositionSnapshot) {
        self.position_history.push_back(position);
        while self.position_history.len() > MAX_POSITION_HISTORY {
            self.position_history.pop_front();
        }
    }
}

fn validate_order(request: &PaperOrderRequest, price: f64) -> Result<(), PaperError> {
    if request.inst_id.trim().is_empty() {
        return Err(PaperError::EmptyInstrument);
    }
    if price <= 0.0 || !price.is_finite() {
        return Err(PaperError::PriceUnavailable(request.inst_id.clone()));
    }
    if request.margin <= 0.0 || !request.margin.is_finite() {
        return Err(PaperError::InvalidMargin);
    }
    if request.leverage < 1.0 || request.leverage > MAX_LEVERAGE || !request.leverage.is_finite() {
        return Err(PaperError::InvalidLeverage);
    }
    Ok(())
}

impl PaperMetadata {
    fn from_request(request: &PaperOrderRequest) -> Self {
        let version_code = request
            .version_code
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_VERSION_CODE.to_string());
        let run_id = request
            .run_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_RUN_ID.to_string());
        let primary_signal = request
            .primary_signal
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "manual".to_string());
        let reason = request
            .strategy_reason
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("{version_code} {primary_signal}"));

        Self {
            source: DEFAULT_SOURCE.to_string(),
            strategy_name: DEFAULT_STRATEGY_NAME.to_string(),
            strategy_version: version_code.clone(),
            version_code,
            run_id,
            primary_signal,
            reason: reason.clone(),
            tags: request.tags.clone(),
            risk_flags: request.risk_flags.clone(),
            risk_guard_decision: request.risk_guard_decision.clone(),
            strategy_reason: reason,
            config_hash: request.config_hash.clone().unwrap_or_default(),
        }
    }
}

fn build_strategy_stats(
    history: &VecDeque<PaperClosedPositionSnapshot>,
) -> Vec<PaperStrategyStats> {
    let mut groups: BTreeMap<(String, String), Vec<PaperClosedPositionSnapshot>> = BTreeMap::new();
    for position in history {
        groups
            .entry((
                position.strategy_name.clone(),
                position.strategy_version.clone(),
            ))
            .or_default()
            .push(position.clone());
    }

    groups
        .into_iter()
        .map(|((strategy_name, strategy_version), group)| {
            let winners = group
                .iter()
                .filter(|position| position.realized_pnl > 0.0)
                .cloned()
                .collect::<Vec<_>>();
            let losers = group
                .iter()
                .filter(|position| position.realized_pnl < 0.0)
                .cloned()
                .collect::<Vec<_>>();
            let gross_profit = winners
                .iter()
                .map(|position| position.realized_pnl)
                .sum::<f64>();
            let gross_loss_abs = losers
                .iter()
                .map(|position| position.realized_pnl)
                .sum::<f64>()
                .abs();
            let first_trade_ts_ms = group.iter().map(|position| position.opened_at_ms).min();
            let last_trade_ts_ms = group.iter().map(|position| position.closed_at_ms).max();

            PaperStrategyStats {
                strategy_name,
                strategy_version,
                total_trades: group.len() * 2,
                closed_position_count: group.len(),
                winning_closed_position_count: winners.len(),
                losing_closed_position_count: losers.len(),
                win_rate: ratio(winners.len(), group.len()),
                realized_pnl: group.iter().map(|position| position.realized_pnl).sum(),
                total_fees: group.iter().map(|position| position.fees).sum(),
                first_trade_ts_ms,
                last_trade_ts_ms,
                running_duration_ms: first_trade_ts_ms
                    .zip(last_trade_ts_ms)
                    .map(|(start, end)| end.saturating_sub(start)),
                average_holding_duration_ms: average(
                    group.iter().map(|position| position.duration_ms as f64),
                ),
                average_position_pnl: average(group.iter().map(|position| position.realized_pnl)),
                profit_factor: if gross_profit > 0.0 && gross_loss_abs > 0.0 {
                    Some(gross_profit / gross_loss_abs)
                } else {
                    None
                },
                largest_winning_pnl: max_number(
                    winners.iter().map(|position| position.realized_pnl),
                ),
                largest_losing_pnl: min_number(losers.iter().map(|position| position.realized_pnl)),
            }
        })
        .collect()
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut count = 0_usize;
    let mut total = 0.0;
    for value in values {
        count += 1;
        total += value;
    }
    (count > 0).then_some(total / count as f64)
}

fn max_number(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.max_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
}

fn min_number(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal))
}

#[cfg(test)]
mod tests {
    use super::*;

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
                trend_score: crate::domain::Score {
                    value: 0,
                    direction: crate::domain::Direction::Neutral,
                    reasons: Vec::new(),
                },
                range_score: crate::domain::Score {
                    value: 0,
                    direction: crate::domain::Direction::Neutral,
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

    #[test]
    fn opens_and_marks_long_position() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest {
                    inst_id: "BTC-USDT-SWAP".to_string(),
                    side: PaperSide::Long,
                    margin: 100.0,
                    leverage: 10.0,
                    ..PaperOrderRequest::default()
                },
                50_000.0,
                10_000.0,
                1,
            )
            .unwrap();

        let snapshot = state.snapshot(&prices("BTC-USDT-SWAP", 51_000.0));
        assert_eq!(snapshot.used_margin, 100.0);
        assert_eq!(snapshot.positions.len(), 1);
        assert!((snapshot.unrealized_pnl - 20.0).abs() < f64::EPSILON);
        assert!((snapshot.available_balance - 9_920.0).abs() < f64::EPSILON);
    }

    #[test]
    fn closes_short_position_into_realized_pnl() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest {
                    inst_id: "ETH-USDT-SWAP".to_string(),
                    side: PaperSide::Short,
                    margin: 200.0,
                    leverage: 5.0,
                    ..PaperOrderRequest::default()
                },
                2_000.0,
                10_000.0,
                1,
            )
            .unwrap();

        state.close("ETH-USDT-SWAP", 1_900.0, 2).unwrap();

        let snapshot = state.snapshot(&prices("ETH-USDT-SWAP", 1_900.0));
        assert!(snapshot.positions.is_empty());
        assert!((snapshot.realized_pnl - 50.0).abs() < f64::EPSILON);
        assert!((snapshot.equity - 10_050.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_opposite_position_without_close() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest {
                    inst_id: "SOL-USDT-SWAP".to_string(),
                    side: PaperSide::Long,
                    margin: 100.0,
                    leverage: 3.0,
                    ..PaperOrderRequest::default()
                },
                100.0,
                10_000.0,
                1,
            )
            .unwrap();

        let error = state
            .open(
                PaperOrderRequest {
                    inst_id: "SOL-USDT-SWAP".to_string(),
                    side: PaperSide::Short,
                    margin: 100.0,
                    leverage: 3.0,
                    ..PaperOrderRequest::default()
                },
                100.0,
                10_000.0,
                2,
            )
            .unwrap_err();

        assert!(matches!(error, PaperError::OppositePosition));
    }
}
