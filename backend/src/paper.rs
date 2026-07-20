use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::domain::SymbolSnapshot;
use crate::persistence::PersistenceHealthSnapshot;
use crate::strategy_identity::{StrategyIdentity, INITIAL_RUN_ID};
use crate::time_regime::TradeTag;

const DEFAULT_INITIAL_BALANCE: f64 = 10_000.0;
const DEFAULT_FEE_RATE: f64 = 0.0005;
const DEFAULT_SLIPPAGE_RATE: f64 = 0.0002;
const MAX_LEVERAGE: f64 = 50.0;
pub const DEFAULT_AUTO_STOP_LOSS_MARGIN_RETURN: f64 = -0.30;
pub const DEFAULT_AUTO_TAKE_PROFIT_MARGIN_RETURN: f64 = 0.40;
pub const SCALPING_OPTIMIZATION_SOURCE: &str = "scalping_optimization_design";
pub const SCALPING_OPTIMIZATION_NAME: &str = "Scalping Optimization Design";
pub const SCALPING_OPTIMIZATION_VERSION: &str = "v0.1.3";

pub trait PaperMarkPrice {
    fn paper_mark_price(&self) -> f64;
}

impl PaperMarkPrice for f64 {
    fn paper_mark_price(&self) -> f64 {
        *self
    }
}

impl PaperMarkPrice for SymbolSnapshot {
    fn paper_mark_price(&self) -> f64 {
        self.price
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrderRequest {
    pub inst_id: String,
    pub side: PaperSide,
    pub margin: f64,
    pub leverage: f64,
    #[serde(default)]
    pub stop_loss: Option<f64>,
    #[serde(default)]
    pub take_profit: Option<f64>,
    #[serde(default)]
    pub expire_at_ms: Option<i64>,
    #[serde(default)]
    pub primary_signal: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub signal_tags: Vec<String>,
}

impl PaperOrderRequest {
    pub fn manual(inst_id: impl Into<String>, side: PaperSide, margin: f64, leverage: f64) -> Self {
        Self {
            inst_id: inst_id.into(),
            side,
            margin,
            leverage,
            stop_loss: None,
            take_profit: None,
            expire_at_ms: None,
            primary_signal: None,
            reason: None,
            signal_tags: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn automatic(
        inst_id: impl Into<String>,
        side: PaperSide,
        margin: f64,
        leverage: f64,
        stop_loss: f64,
        take_profit: f64,
        expire_at_ms: Option<i64>,
        primary_signal: impl Into<String>,
        reason: impl Into<String>,
        tags: Vec<String>,
    ) -> Self {
        Self {
            inst_id: inst_id.into(),
            side,
            margin,
            leverage,
            stop_loss: Some(stop_loss),
            take_profit: Some(take_profit),
            expire_at_ms,
            primary_signal: Some(primary_signal.into()),
            reason: Some(reason.into()),
            signal_tags: tags,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperAccountSnapshot {
    pub mode: String,
    pub initial_balance: f64,
    pub current_strategy_source: String,
    pub current_strategy_name: String,
    pub current_strategy_version: String,
    pub strategy_version: String,
    pub strategy_build_id: String,
    pub config_hash: String,
    pub run_id: String,
    pub persistence: PersistenceHealthSnapshot,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub primary_signal: String,
    pub reason: String,
    pub fee: f64,
    pub config_hash: String,
    pub signal_tags: Vec<String>,
    pub tags: Vec<TradeTag>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub expire_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub primary_signal: String,
    pub reason: String,
    pub close_source: String,
    pub close_reason: String,
    pub signal_tags: Vec<String>,
    pub tags: Vec<TradeTag>,
    pub open_tags: Vec<TradeTag>,
    pub close_tags: Vec<TradeTag>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub expire_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTrade {
    pub id: u64,
    pub inst_id: String,
    pub side: PaperSide,
    pub action: PaperTradeAction,
    #[serde(default = "manual_trade_source")]
    pub source: String,
    #[serde(default)]
    pub strategy_name: String,
    #[serde(default)]
    pub strategy_version: String,
    #[serde(default)]
    pub primary_signal: String,
    #[serde(default = "manual_trade_reason")]
    pub reason: String,
    pub price: f64,
    pub qty: f64,
    pub margin: f64,
    pub notional: f64,
    #[serde(default)]
    pub fee: f64,
    #[serde(default)]
    pub slippage_rate: f64,
    #[serde(default)]
    pub signal_tags: Vec<String>,
    #[serde(default)]
    pub tags: Vec<TradeTag>,
    #[serde(default)]
    pub stop_loss: Option<f64>,
    #[serde(default)]
    pub take_profit: Option<f64>,
    #[serde(default)]
    pub expire_at_ms: Option<i64>,
    pub realized_pnl: f64,
    pub ts_ms: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperState {
    #[serde(default = "default_strategy_identity")]
    strategy_identity: StrategyIdentity,
    #[serde(default = "default_run_id")]
    run_id: String,
    #[serde(default = "default_initial_balance")]
    initial_balance: f64,
    #[serde(default)]
    realized_pnl: f64,
    #[serde(default = "default_fee_rate")]
    fee_rate: f64,
    #[serde(default = "default_slippage_rate")]
    slippage_rate: f64,
    #[serde(default)]
    positions: BTreeMap<String, PaperPosition>,
    #[serde(default)]
    closed_positions: VecDeque<PaperClosedPositionSnapshot>,
    #[serde(default)]
    trades: VecDeque<PaperTrade>,
    #[serde(default = "default_next_trade_id")]
    next_trade_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PaperPosition {
    inst_id: String,
    side: PaperSide,
    qty: f64,
    entry_price: f64,
    margin: f64,
    leverage: f64,
    notional: f64,
    opened_at_ms: i64,
    #[serde(default)]
    open_fee: f64,
    #[serde(default = "manual_trade_source")]
    source: String,
    #[serde(default)]
    strategy_name: String,
    #[serde(default)]
    strategy_version: String,
    #[serde(default)]
    primary_signal: String,
    #[serde(default = "manual_trade_reason")]
    reason: String,
    #[serde(default)]
    signal_tags: Vec<String>,
    #[serde(default)]
    tags: Vec<TradeTag>,
    #[serde(default)]
    stop_loss: Option<f64>,
    #[serde(default)]
    take_profit: Option<f64>,
    #[serde(default)]
    expire_at_ms: Option<i64>,
}

impl Default for PaperState {
    fn default() -> Self {
        Self {
            strategy_identity: StrategyIdentity::restored_v3(),
            run_id: INITIAL_RUN_ID.to_string(),
            initial_balance: DEFAULT_INITIAL_BALANCE,
            realized_pnl: 0.0,
            fee_rate: DEFAULT_FEE_RATE,
            slippage_rate: DEFAULT_SLIPPAGE_RATE,
            positions: BTreeMap::new(),
            closed_positions: VecDeque::new(),
            trades: VecDeque::new(),
            next_trade_id: 1,
        }
    }
}

impl PaperState {
    pub fn fresh_restored_v3(strategy_identity: StrategyIdentity) -> Self {
        Self {
            strategy_identity,
            ..Self::default()
        }
    }

    pub fn strategy_identity(&self) -> &StrategyIdentity {
        &self.strategy_identity
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn next_trade_id(&self) -> u64 {
        self.next_trade_id
    }

    pub fn snapshot<T: PaperMarkPrice>(
        &self,
        prices: &BTreeMap<String, T>,
    ) -> PaperAccountSnapshot {
        let positions: Vec<_> = self
            .positions
            .values()
            .map(|position| self.position_snapshot(position, prices))
            .collect();
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
        let position_history: Vec<_> = self.closed_positions.iter().rev().cloned().collect();
        let closed_position_count = position_history.len();
        let winning_closed_position_count = position_history
            .iter()
            .filter(|position| position.realized_pnl > 0.0)
            .count();
        let losing_closed_position_count = position_history
            .iter()
            .filter(|position| position.realized_pnl < 0.0)
            .count();
        let win_rate = ratio(winning_closed_position_count, closed_position_count);
        let average_holding_duration_ms = average(
            position_history
                .iter()
                .map(|position| position.duration_ms as f64),
        );
        let average_closed_position_pnl = average(
            position_history
                .iter()
                .map(|position| position.realized_pnl),
        );
        let average_winning_pnl = average(
            position_history
                .iter()
                .filter(|position| position.realized_pnl > 0.0)
                .map(|position| position.realized_pnl),
        );
        let average_losing_pnl = average(
            position_history
                .iter()
                .filter(|position| position.realized_pnl < 0.0)
                .map(|position| position.realized_pnl),
        );
        let gross_profit = position_history
            .iter()
            .filter(|position| position.realized_pnl > 0.0)
            .map(|position| position.realized_pnl)
            .sum::<f64>();
        let gross_loss = position_history
            .iter()
            .filter(|position| position.realized_pnl < 0.0)
            .map(|position| position.realized_pnl.abs())
            .sum::<f64>();

        PaperAccountSnapshot {
            mode: "paper".to_string(),
            initial_balance: self.initial_balance,
            current_strategy_source: SCALPING_OPTIMIZATION_SOURCE.to_string(),
            current_strategy_name: SCALPING_OPTIMIZATION_NAME.to_string(),
            current_strategy_version: SCALPING_OPTIMIZATION_VERSION.to_string(),
            strategy_version: self.strategy_identity.version_code.clone(),
            strategy_build_id: self.strategy_identity.strategy_build_id.clone(),
            config_hash: self.strategy_identity.config_hash.clone(),
            run_id: self.run_id.clone(),
            persistence: PersistenceHealthSnapshot::default(),
            fee_rate: self.fee_rate,
            slippage_rate: self.slippage_rate,
            total_fees: self.trades.iter().map(|trade| trade.fee).sum(),
            total_trades: self.trades.len(),
            closed_position_count,
            winning_closed_position_count,
            losing_closed_position_count,
            win_rate,
            average_holding_duration_ms,
            average_closed_position_pnl,
            average_winning_pnl,
            average_losing_pnl,
            profit_factor: profit_factor(gross_profit, gross_loss),
            largest_winning_pnl: position_history
                .iter()
                .filter(|position| position.realized_pnl > 0.0)
                .map(|position| position.realized_pnl)
                .max_by(f64::total_cmp),
            largest_losing_pnl: position_history
                .iter()
                .filter(|position| position.realized_pnl < 0.0)
                .map(|position| position.realized_pnl)
                .min_by(f64::total_cmp),
            strategy_stats: strategy_stats(&position_history, self.trades.iter()),
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

    pub fn open_position_inst_ids(&self) -> Vec<String> {
        self.positions.keys().cloned().collect()
    }

    pub fn open(
        &mut self,
        request: PaperOrderRequest,
        price: f64,
        available_balance: f64,
        ts_ms: i64,
    ) -> Result<PaperTrade, PaperError> {
        let is_automatic = request.primary_signal.is_some();
        let source = if is_automatic {
            SCALPING_OPTIMIZATION_SOURCE
        } else {
            "manual"
        };
        let reason = request.reason.clone().unwrap_or_else(|| source.to_string());
        self.open_with_meta_and_tags(
            request,
            price,
            available_balance,
            ts_ms,
            source,
            &reason,
            Vec::new(),
        )
    }

    pub fn open_with_meta(
        &mut self,
        request: PaperOrderRequest,
        price: f64,
        available_balance: f64,
        ts_ms: i64,
        source: &str,
        reason: &str,
    ) -> Result<PaperTrade, PaperError> {
        self.open_with_meta_and_tags(
            request,
            price,
            available_balance,
            ts_ms,
            source,
            reason,
            Vec::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn open_with_meta_and_tags(
        &mut self,
        request: PaperOrderRequest,
        price: f64,
        available_balance: f64,
        ts_ms: i64,
        source: &str,
        reason: &str,
        tags: Vec<TradeTag>,
    ) -> Result<PaperTrade, PaperError> {
        validate_order(&request, price)?;
        let execution_price = self.execution_price(request.side, PaperTradeAction::Open, price);
        let fee = request.margin * request.leverage * self.fee_rate;
        if available_balance < request.margin + fee {
            return Err(PaperError::InsufficientBalance);
        }

        let qty = request.margin * request.leverage / execution_price;
        let notional = qty * execution_price;
        let strategy_name = strategy_name_for_source(source);
        let strategy_version = strategy_version_for_source(source);
        let primary_signal = request.primary_signal.clone().unwrap_or_default();
        self.realized_pnl -= fee;

        if let Some(existing) = self.positions.get_mut(&request.inst_id) {
            if existing.side != request.side {
                self.realized_pnl += fee;
                return Err(PaperError::OppositePosition);
            }
            existing.qty += qty;
            existing.margin += request.margin;
            existing.notional += notional;
            existing.entry_price = existing.notional / existing.qty;
            existing.leverage = existing.notional / existing.margin;
            existing.open_fee += fee;
            existing.signal_tags.extend(request.signal_tags.clone());
            existing.tags.extend(tags.clone());
        } else {
            self.positions.insert(
                request.inst_id.clone(),
                PaperPosition {
                    inst_id: request.inst_id.clone(),
                    side: request.side,
                    qty,
                    entry_price: execution_price,
                    margin: request.margin,
                    leverage: request.leverage,
                    notional,
                    opened_at_ms: ts_ms,
                    open_fee: fee,
                    source: source.to_string(),
                    strategy_name: strategy_name.clone(),
                    strategy_version: strategy_version.clone(),
                    primary_signal: primary_signal.clone(),
                    reason: reason.to_string(),
                    signal_tags: request.signal_tags.clone(),
                    tags: tags.clone(),
                    stop_loss: request.stop_loss,
                    take_profit: request.take_profit,
                    expire_at_ms: request.expire_at_ms,
                },
            );
        }

        let trade = PaperTrade {
            id: self.allocate_trade_id(),
            inst_id: request.inst_id,
            side: request.side,
            action: PaperTradeAction::Open,
            source: source.to_string(),
            strategy_name,
            strategy_version,
            primary_signal,
            reason: reason.to_string(),
            price: execution_price,
            qty,
            margin: request.margin,
            notional,
            fee,
            slippage_rate: self.slippage_rate,
            signal_tags: request.signal_tags,
            tags,
            stop_loss: request.stop_loss,
            take_profit: request.take_profit,
            expire_at_ms: request.expire_at_ms,
            realized_pnl: -fee,
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
        self.close_with_meta(inst_id, price, ts_ms, "manual", "manual")
    }

    pub fn close_with_meta(
        &mut self,
        inst_id: &str,
        price: f64,
        ts_ms: i64,
        source: &str,
        reason: &str,
    ) -> Result<PaperTrade, PaperError> {
        self.close_with_meta_and_tags(inst_id, price, ts_ms, source, reason, Vec::new())
    }

    pub fn close_with_meta_and_tags(
        &mut self,
        inst_id: &str,
        price: f64,
        ts_ms: i64,
        source: &str,
        reason: &str,
        tags: Vec<TradeTag>,
    ) -> Result<PaperTrade, PaperError> {
        if price <= 0.0 || !price.is_finite() {
            return Err(PaperError::PriceUnavailable(inst_id.to_string()));
        }
        let position = self
            .positions
            .remove(inst_id)
            .ok_or_else(|| PaperError::PositionNotFound(inst_id.to_string()))?;
        let execution_price = self.execution_price(position.side, PaperTradeAction::Close, price);
        let close_notional = position.qty * execution_price;
        let fee = close_notional * self.fee_rate;
        let gross_pnl = position
            .side
            .pnl(position.entry_price, execution_price, position.qty);
        let realized_pnl = gross_pnl - fee;
        self.realized_pnl += realized_pnl;

        let trade = PaperTrade {
            id: self.allocate_trade_id(),
            inst_id: position.inst_id.clone(),
            side: position.side,
            action: PaperTradeAction::Close,
            source: source.to_string(),
            strategy_name: strategy_name_for_source(source),
            strategy_version: strategy_version_for_source(source),
            primary_signal: position.primary_signal.clone(),
            reason: reason.to_string(),
            price: execution_price,
            qty: position.qty,
            margin: position.margin,
            notional: close_notional,
            fee,
            slippage_rate: self.slippage_rate,
            signal_tags: position.signal_tags.clone(),
            tags: tags.clone(),
            stop_loss: position.stop_loss,
            take_profit: position.take_profit,
            expire_at_ms: position.expire_at_ms,
            realized_pnl,
            ts_ms,
        };
        let all_tags = position
            .tags
            .iter()
            .cloned()
            .chain(tags.iter().cloned())
            .collect();
        self.closed_positions
            .push_back(PaperClosedPositionSnapshot {
                id: trade.id,
                inst_id: position.inst_id,
                side: position.side,
                qty: position.qty,
                entry_price: position.entry_price,
                exit_price: execution_price,
                margin: position.margin,
                leverage: position.leverage,
                notional: position.notional,
                fees: position.open_fee + fee,
                realized_pnl: gross_pnl - position.open_fee - fee,
                pnl_pct: (gross_pnl - position.open_fee - fee) / position.margin,
                opened_at_ms: position.opened_at_ms,
                closed_at_ms: ts_ms,
                duration_ms: ts_ms.saturating_sub(position.opened_at_ms),
                source: position.source,
                strategy_name: position.strategy_name,
                strategy_version: position.strategy_version,
                primary_signal: position.primary_signal,
                reason: position.reason,
                close_source: source.to_string(),
                close_reason: reason.to_string(),
                signal_tags: position.signal_tags,
                tags: all_tags,
                open_tags: position.tags,
                close_tags: tags,
                stop_loss: position.stop_loss,
                take_profit: position.take_profit,
                expire_at_ms: position.expire_at_ms,
            });
        self.push_trade(trade.clone());
        Ok(trade)
    }

    fn position_snapshot<T: PaperMarkPrice>(
        &self,
        position: &PaperPosition,
        prices: &BTreeMap<String, T>,
    ) -> PaperPositionSnapshot {
        let mark_price = prices
            .get(&position.inst_id)
            .map(PaperMarkPrice::paper_mark_price)
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
            source: position.source.clone(),
            strategy_name: position.strategy_name.clone(),
            strategy_version: position.strategy_version.clone(),
            primary_signal: position.primary_signal.clone(),
            reason: position.reason.clone(),
            fee: position.open_fee,
            config_hash: self.strategy_identity.config_hash.clone(),
            signal_tags: position.signal_tags.clone(),
            tags: position.tags.clone(),
            stop_loss: position.stop_loss,
            take_profit: position.take_profit,
            expire_at_ms: position.expire_at_ms,
        }
    }

    fn allocate_trade_id(&mut self) -> u64 {
        let id = self.next_trade_id;
        self.next_trade_id += 1;
        id
    }

    fn push_trade(&mut self, trade: PaperTrade) {
        self.trades.push_back(trade);
    }

    fn execution_price(&self, side: PaperSide, action: PaperTradeAction, price: f64) -> f64 {
        let multiplier = match (side, action) {
            (PaperSide::Long, PaperTradeAction::Open)
            | (PaperSide::Short, PaperTradeAction::Close) => 1.0 + self.slippage_rate,
            (PaperSide::Long, PaperTradeAction::Close)
            | (PaperSide::Short, PaperTradeAction::Open) => 1.0 - self.slippage_rate,
        };
        price * multiplier
    }
}

pub fn automatic_trigger_prices(entry_price: f64, side: PaperSide, leverage: f64) -> (f64, f64) {
    let direction = match side {
        PaperSide::Long => 1.0,
        PaperSide::Short => -1.0,
    };
    let stop_loss =
        entry_price * (1.0 + direction * DEFAULT_AUTO_STOP_LOSS_MARGIN_RETURN / leverage);
    let take_profit =
        entry_price * (1.0 + direction * DEFAULT_AUTO_TAKE_PROFIT_MARGIN_RETURN / leverage);
    (stop_loss, take_profit)
}

fn average(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut count = 0usize;
    let mut total = 0.0;
    for value in values {
        count += 1;
        total += value;
    }
    (count > 0).then_some(total / count as f64)
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn profit_factor(gross_profit: f64, gross_loss: f64) -> Option<f64> {
    (gross_profit > 0.0 && gross_loss > 0.0).then_some(gross_profit / gross_loss)
}

fn strategy_stats<'a>(
    position_history: &[PaperClosedPositionSnapshot],
    trades: impl Iterator<Item = &'a PaperTrade>,
) -> Vec<PaperStrategyStats> {
    let mut open_trades = BTreeMap::<(String, String), Vec<&PaperTrade>>::new();
    for trade in trades.filter(|trade| trade.action == PaperTradeAction::Open) {
        open_trades
            .entry((
                normalized(&trade.strategy_name),
                normalized(&trade.strategy_version),
            ))
            .or_default()
            .push(trade);
    }

    let mut closed_by_strategy =
        BTreeMap::<(String, String), Vec<&PaperClosedPositionSnapshot>>::new();
    for position in position_history {
        closed_by_strategy
            .entry((
                normalized(&position.strategy_name),
                normalized(&position.strategy_version),
            ))
            .or_default()
            .push(position);
    }
    for key in closed_by_strategy.keys() {
        open_trades.entry(key.clone()).or_default();
    }

    open_trades
        .into_iter()
        .map(|((strategy_name, strategy_version), trades)| {
            let positions = closed_by_strategy
                .get(&(strategy_name.clone(), strategy_version.clone()))
                .cloned()
                .unwrap_or_default();
            let winning = positions
                .iter()
                .filter(|position| position.realized_pnl > 0.0)
                .count();
            let losing = positions
                .iter()
                .filter(|position| position.realized_pnl < 0.0)
                .count();
            let gross_profit = positions
                .iter()
                .filter(|position| position.realized_pnl > 0.0)
                .map(|position| position.realized_pnl)
                .sum::<f64>();
            let gross_loss = positions
                .iter()
                .filter(|position| position.realized_pnl < 0.0)
                .map(|position| position.realized_pnl.abs())
                .sum::<f64>();
            let first_trade_ts_ms = trades.iter().map(|trade| trade.ts_ms).min();
            let last_trade_ts_ms = trades
                .iter()
                .map(|trade| trade.ts_ms)
                .chain(positions.iter().map(|position| position.closed_at_ms))
                .max();
            PaperStrategyStats {
                strategy_name,
                strategy_version,
                total_trades: trades.len(),
                closed_position_count: positions.len(),
                winning_closed_position_count: winning,
                losing_closed_position_count: losing,
                win_rate: ratio(winning, positions.len()),
                realized_pnl: positions.iter().map(|position| position.realized_pnl).sum(),
                total_fees: positions.iter().map(|position| position.fees).sum(),
                first_trade_ts_ms,
                last_trade_ts_ms,
                running_duration_ms: first_trade_ts_ms
                    .zip(last_trade_ts_ms)
                    .map(|(first, last)| last.saturating_sub(first)),
                average_holding_duration_ms: average(
                    positions.iter().map(|position| position.duration_ms as f64),
                ),
                average_position_pnl: average(
                    positions.iter().map(|position| position.realized_pnl),
                ),
                profit_factor: profit_factor(gross_profit, gross_loss),
                largest_winning_pnl: positions
                    .iter()
                    .filter(|position| position.realized_pnl > 0.0)
                    .map(|position| position.realized_pnl)
                    .max_by(f64::total_cmp),
                largest_losing_pnl: positions
                    .iter()
                    .filter(|position| position.realized_pnl < 0.0)
                    .map(|position| position.realized_pnl)
                    .min_by(f64::total_cmp),
            }
        })
        .collect()
}

fn strategy_name_for_source(source: &str) -> String {
    if matches!(source, SCALPING_OPTIMIZATION_SOURCE | "auto" | "auto_v1") {
        SCALPING_OPTIMIZATION_NAME.to_string()
    } else {
        source.to_string()
    }
}

fn strategy_version_for_source(source: &str) -> String {
    if matches!(source, SCALPING_OPTIMIZATION_SOURCE | "auto" | "auto_v1") {
        SCALPING_OPTIMIZATION_VERSION.to_string()
    } else {
        source.to_string()
    }
}

fn normalized(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        "unknown".to_string()
    } else {
        value.to_string()
    }
}

fn default_initial_balance() -> f64 {
    DEFAULT_INITIAL_BALANCE
}

fn default_fee_rate() -> f64 {
    DEFAULT_FEE_RATE
}

fn default_slippage_rate() -> f64 {
    DEFAULT_SLIPPAGE_RATE
}

fn default_next_trade_id() -> u64 {
    1
}

fn default_strategy_identity() -> StrategyIdentity {
    StrategyIdentity::restored_v3()
}

fn default_run_id() -> String {
    INITIAL_RUN_ID.to_string()
}

fn manual_trade_source() -> String {
    "manual".to_string()
}

fn manual_trade_reason() -> String {
    "manual".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time_regime::{TradeTag, TradeTagKind};

    fn neutral_symbol(inst_id: &str, price: f64) -> SymbolSnapshot {
        SymbolSnapshot {
            inst_id: inst_id.to_string(),
            price,
            change_5m_pct: 0.0,
            change_15m_pct: 0.0,
            change_1h_pct: 0.0,
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
            scalping_metrics: Default::default(),
            fvgs: Vec::new(),
            levels: Vec::new(),
            pattern_signals: Vec::new(),
            updated_at_ms: 1,
        }
    }

    fn prices(inst_id: &str, price: f64) -> BTreeMap<String, SymbolSnapshot> {
        BTreeMap::from([(inst_id.to_string(), neutral_symbol(inst_id, price))])
    }

    #[test]
    fn entry_and_exit_apply_fee_and_adverse_slippage() {
        let mut state = PaperState::default();
        let request = PaperOrderRequest::automatic(
            "ETH-USDT-SWAP",
            PaperSide::Long,
            300.0,
            20.0,
            985.0,
            1_020.0,
            None,
            "trend_long",
            "paper fee fixture",
            vec!["long".to_string()],
        );
        state.open(request, 1_000.0, 10_000.0, 1).unwrap();
        let opened = state.snapshot(&BTreeMap::from([(
            "ETH-USDT-SWAP".to_string(),
            neutral_symbol("ETH-USDT-SWAP", 1_000.0),
        )]));
        assert_eq!(opened.fee_rate, 0.0005);
        assert_eq!(opened.slippage_rate, 0.0002);
        assert!(opened.total_fees > 0.0);

        let closed = state
            .close_with_meta("ETH-USDT-SWAP", 1_020.0, 2, "auto", "take profit")
            .unwrap();
        assert!(closed.price < 1_020.0);
        assert!(closed.fee > 0.0);
    }

    #[test]
    fn closed_history_preserves_strategy_and_trigger_metadata() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest::automatic(
                    "ETH-USDT-SWAP",
                    PaperSide::Long,
                    300.0,
                    20.0,
                    985.0,
                    1_020.0,
                    None,
                    "trend_long",
                    "paper history fixture",
                    vec!["long".to_string()],
                ),
                1_000.0,
                10_000.0,
                1,
            )
            .unwrap();
        state
            .close_with_meta("ETH-USDT-SWAP", 985.0, 2, "auto", "stop loss")
            .unwrap();
        let snapshot = state.snapshot(&BTreeMap::from([(
            "ETH-USDT-SWAP".to_string(),
            neutral_symbol("ETH-USDT-SWAP", 985.0),
        )]));
        assert_eq!(snapshot.position_history[0].strategy_version, "v0.1.3");
        assert_eq!(snapshot.position_history[0].primary_signal, "trend_long");
        assert_eq!(snapshot.position_history[0].stop_loss, Some(985.0));
        assert!(snapshot.position_history[0]
            .close_reason
            .contains("stop loss"));
    }

    #[test]
    fn default_margin_return_thresholds_map_to_20x_trigger_prices() {
        let (long_stop, long_target) = automatic_trigger_prices(1.0, PaperSide::Long, 20.0);
        assert!((long_stop - 0.985).abs() < f64::EPSILON);
        assert!((long_target - 1.02).abs() < f64::EPSILON);

        let (short_stop, short_target) = automatic_trigger_prices(1.0, PaperSide::Short, 20.0);
        assert!((short_stop - 1.015).abs() < f64::EPSILON);
        assert!((short_target - 0.98).abs() < f64::EPSILON);
    }

    #[test]
    fn serialized_state_restores_position_metadata_and_next_trade_id() {
        let mut state = PaperState::default();
        let time_tag = TradeTag {
            kind: TradeTagKind::RequiresHighConfidence,
            label: "requires high confidence".to_string(),
            score_impact: 0,
            reason: "test time gate".to_string(),
            ts_ms: 1,
        };
        state
            .open_with_meta_and_tags(
                PaperOrderRequest::automatic(
                    "SOL-USDT-SWAP",
                    PaperSide::Short,
                    100.0,
                    20.0,
                    101.5,
                    98.0,
                    Some(60_001),
                    "range_short",
                    "serialized fixture",
                    vec!["short".to_string()],
                ),
                100.0,
                10_000.0,
                1,
                SCALPING_OPTIMIZATION_SOURCE,
                "serialized fixture",
                vec![time_tag],
            )
            .unwrap();

        let raw = serde_json::to_string(&state).unwrap();
        let mut restored: PaperState = serde_json::from_str(&raw).unwrap();
        let open = restored.snapshot(&prices("SOL-USDT-SWAP", 100.0));
        assert_eq!(open.positions[0].expire_at_ms, Some(60_001));
        assert_eq!(open.positions[0].primary_signal, "range_short");
        assert_eq!(open.positions[0].tags.len(), 1);

        let close = restored
            .close_with_meta("SOL-USDT-SWAP", 98.0, 60_001, "auto", "expired")
            .unwrap();
        assert_eq!(close.id, 2);
        let closed = restored.snapshot(&prices("SOL-USDT-SWAP", 98.0));
        assert_eq!(closed.position_history[0].expire_at_ms, Some(60_001));
        assert_eq!(closed.position_history[0].close_reason, "expired");
    }

    #[test]
    fn close_reason_is_never_inferred_from_profit_or_loss() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest::automatic(
                    "BTC-USDT-SWAP",
                    PaperSide::Long,
                    100.0,
                    20.0,
                    98.5,
                    102.0,
                    Some(10),
                    "trend_long",
                    "expiry fixture",
                    Vec::new(),
                ),
                100.0,
                10_000.0,
                1,
            )
            .unwrap();
        state
            .close_with_meta("BTC-USDT-SWAP", 99.0, 10, "auto", "expired")
            .unwrap();

        let snapshot = state.snapshot(&prices("BTC-USDT-SWAP", 99.0));
        assert!(snapshot.position_history[0].realized_pnl < 0.0);
        assert_eq!(snapshot.position_history[0].close_reason, "expired");
    }

    #[test]
    fn manual_request_json_keeps_the_original_four_field_contract() {
        let request: PaperOrderRequest = serde_json::from_str(
            r#"{"inst_id":"BTC-USDT-SWAP","side":"long","margin":100.0,"leverage":10.0}"#,
        )
        .unwrap();
        assert!(request.stop_loss.is_none());
        assert!(request.primary_signal.is_none());
        assert!(request.signal_tags.is_empty());
    }

    #[test]
    fn opens_and_marks_long_position() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest::manual("BTC-USDT-SWAP", PaperSide::Long, 100.0, 10.0),
                50_000.0,
                10_000.0,
                1,
            )
            .unwrap();

        let snapshot = state.snapshot(&prices("BTC-USDT-SWAP", 51_000.0));
        assert_eq!(snapshot.used_margin, 100.0);
        assert_eq!(snapshot.positions.len(), 1);
        assert!(snapshot.unrealized_pnl > 19.0);
        assert!(snapshot.unrealized_pnl < 20.0);
        assert!(snapshot.available_balance > 9_918.0);
        assert!(snapshot.available_balance < 9_920.0);
    }

    #[test]
    fn closes_short_position_into_realized_pnl() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest::manual("ETH-USDT-SWAP", PaperSide::Short, 200.0, 5.0),
                2_000.0,
                10_000.0,
                1,
            )
            .unwrap();

        state.close("ETH-USDT-SWAP", 1_900.0, 2).unwrap();

        let snapshot = state.snapshot(&prices("ETH-USDT-SWAP", 1_900.0));
        assert!(snapshot.positions.is_empty());
        assert!(snapshot.realized_pnl > 48.0);
        assert!(snapshot.realized_pnl < 49.0);
        assert_eq!(snapshot.equity, 10_000.0 + snapshot.realized_pnl);
    }

    #[test]
    fn rejects_opposite_position_without_close() {
        let mut state = PaperState::default();
        state
            .open(
                PaperOrderRequest::manual("SOL-USDT-SWAP", PaperSide::Long, 100.0, 3.0),
                100.0,
                10_000.0,
                1,
            )
            .unwrap();

        let error = state
            .open(
                PaperOrderRequest::manual("SOL-USDT-SWAP", PaperSide::Short, 100.0, 3.0),
                100.0,
                10_000.0,
                2,
            )
            .unwrap_err();

        assert!(matches!(error, PaperError::OppositePosition));
    }
}
