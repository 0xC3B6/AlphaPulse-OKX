use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::domain::SymbolSnapshot;

const DEFAULT_INITIAL_BALANCE: f64 = 10_000.0;
const MAX_LEVERAGE: f64 = 50.0;
const MAX_TRADES: usize = 100;

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
}

#[derive(Debug, Clone, Serialize)]
pub struct PaperAccountSnapshot {
    pub mode: &'static str,
    pub initial_balance: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub equity: f64,
    pub used_margin: f64,
    pub available_balance: f64,
    pub positions: Vec<PaperPositionSnapshot>,
    pub trades: Vec<PaperTrade>,
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

#[derive(Debug, Clone)]
pub struct PaperState {
    initial_balance: f64,
    realized_pnl: f64,
    positions: BTreeMap<String, PaperPosition>,
    trades: VecDeque<PaperTrade>,
    next_trade_id: u64,
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
}

impl Default for PaperState {
    fn default() -> Self {
        Self {
            initial_balance: DEFAULT_INITIAL_BALANCE,
            realized_pnl: 0.0,
            positions: BTreeMap::new(),
            trades: VecDeque::new(),
            next_trade_id: 1,
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

        PaperAccountSnapshot {
            mode: "paper",
            initial_balance: self.initial_balance,
            realized_pnl: self.realized_pnl,
            unrealized_pnl,
            equity,
            used_margin,
            available_balance,
            positions,
            trades: self.trades.iter().rev().cloned().collect(),
        }
    }

    pub fn has_open_positions(&self) -> bool {
        !self.positions.is_empty()
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

        let trade = PaperTrade {
            id: self.next_trade_id(),
            inst_id: position.inst_id,
            side: position.side,
            action: PaperTradeAction::Close,
            price,
            qty: position.qty,
            margin: position.margin,
            notional: position.qty * price,
            realized_pnl,
            ts_ms,
        };
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
                },
                100.0,
                10_000.0,
                2,
            )
            .unwrap_err();

        assert!(matches!(error, PaperError::OppositePosition));
    }
}
