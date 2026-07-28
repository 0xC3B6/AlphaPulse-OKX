use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    auto_strategy::{signal_score_for_primary, AutoStrategyCandidate, CandidateDisposition},
    domain::{Direction, SymbolSnapshot},
    paper::{PaperAccountSnapshot, PaperPositionSnapshot, PaperSide},
    strategy_identity::StrategyIdentity,
    time_regime::{TradeTag, TradeTagKind},
};

pub const CANDIDATE_EVENT_BUCKET_MS: i64 = 15 * 60 * 1_000;
pub const POSITION_CONTEXT_BUCKET_MS: i64 = 60 * 60 * 1_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategyCandidateEvent {
    pub event_key: String,
    pub run_id: String,
    pub identity: StrategyIdentity,
    pub bucket_start_ms: i64,
    pub symbol: String,
    pub side: PaperSide,
    pub primary_signal: String,
    pub disposition: CandidateDisposition,
    pub rejection_reason: Option<String>,
    pub ranking: Option<u32>,
    pub first_observed_at_ms: i64,
    pub last_observed_at_ms: i64,
    pub observation_count: u64,
    pub first_score: u8,
    pub high_score: u8,
    pub low_score: u8,
    pub last_score: u8,
    pub reason: String,
    pub planned_margin: f64,
    pub planned_leverage: f64,
    pub planned_stop_loss: Option<f64>,
    pub planned_take_profit: Option<f64>,
    pub session_context: Vec<String>,
    pub event_context: Vec<String>,
    pub risk_tags: Vec<TradeTag>,
    pub feature_snapshot: Value,
    pub market_snapshot: Value,
}

impl StrategyCandidateEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: &StrategyIdentity,
        run_id: &str,
        symbol: &SymbolSnapshot,
        paper: &PaperAccountSnapshot,
        btc: Option<&SymbolSnapshot>,
        candidate: AutoStrategyCandidate,
        observed_at_ms: i64,
    ) -> Self {
        let bucket_start_ms = bucket_start(observed_at_ms, CANDIDATE_EVENT_BUCKET_MS);
        let experiment_key = identity.experiment_key();
        let event_key = format!(
            "{experiment_key}:{run_id}:{bucket_start_ms}:{}:{}:{}:{}",
            symbol.inst_id,
            side_name(candidate.side),
            candidate.primary_signal,
            candidate.disposition.as_str()
        );
        let session_context = candidate
            .tags
            .iter()
            .filter(|tag| is_session_tag(tag.kind))
            .map(|tag| tag.label.clone())
            .collect();
        let event_context = candidate
            .tags
            .iter()
            .filter(|tag| is_event_tag(tag.kind))
            .map(|tag| tag.label.clone())
            .collect();
        Self {
            event_key,
            run_id: run_id.to_string(),
            identity: identity.clone(),
            bucket_start_ms,
            symbol: symbol.inst_id.clone(),
            side: candidate.side,
            primary_signal: candidate.primary_signal,
            disposition: candidate.disposition,
            rejection_reason: candidate.rejection_reason,
            ranking: None,
            first_observed_at_ms: observed_at_ms,
            last_observed_at_ms: observed_at_ms,
            observation_count: 1,
            first_score: candidate.score,
            high_score: candidate.score,
            low_score: candidate.score,
            last_score: candidate.score,
            reason: candidate.reason,
            planned_margin: candidate.planned_margin,
            planned_leverage: candidate.planned_leverage,
            planned_stop_loss: candidate.planned_stop_loss,
            planned_take_profit: candidate.planned_take_profit,
            session_context,
            event_context,
            risk_tags: candidate.tags,
            feature_snapshot: feature_snapshot(symbol),
            market_snapshot: market_snapshot(paper, btc),
        }
    }

    pub fn merge(&mut self, newer: Self) {
        debug_assert_eq!(self.event_key, newer.event_key);
        self.observation_count = self
            .observation_count
            .saturating_add(newer.observation_count);
        self.high_score = self.high_score.max(newer.high_score);
        self.low_score = self.low_score.min(newer.low_score);
        if newer.first_observed_at_ms < self.first_observed_at_ms {
            self.first_observed_at_ms = newer.first_observed_at_ms;
            self.first_score = newer.first_score;
        }
        if newer.last_observed_at_ms >= self.last_observed_at_ms {
            self.last_observed_at_ms = newer.last_observed_at_ms;
            self.last_score = newer.last_score;
            self.rejection_reason = newer.rejection_reason;
            self.ranking = newer.ranking;
            self.reason = newer.reason;
            self.planned_margin = newer.planned_margin;
            self.planned_leverage = newer.planned_leverage;
            self.planned_stop_loss = newer.planned_stop_loss;
            self.planned_take_profit = newer.planned_take_profit;
            self.session_context = newer.session_context;
            self.event_context = newer.event_context;
            self.risk_tags = newer.risk_tags;
            self.feature_snapshot = newer.feature_snapshot;
            self.market_snapshot = newer.market_snapshot;
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PositionContextEventKind {
    Opened,
    Checkpoint,
    ExitSignal,
}

impl PositionContextEventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opened => "opened",
            Self::Checkpoint => "checkpoint",
            Self::ExitSignal => "exit_signal",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PositionContextEvent {
    pub event_key: String,
    pub position_key: String,
    pub run_id: String,
    pub identity: StrategyIdentity,
    pub event_kind: PositionContextEventKind,
    pub bucket_start_ms: i64,
    pub symbol: String,
    pub side: PaperSide,
    pub primary_signal: String,
    pub opened_at_ms: i64,
    pub first_observed_at_ms: i64,
    pub last_observed_at_ms: i64,
    pub observation_count: u64,
    pub position_age_ms: i64,
    pub current_score: Option<u8>,
    pub open_mark_price: f64,
    pub high_mark_price: f64,
    pub low_mark_price: f64,
    pub close_mark_price: f64,
    pub open_pnl: f64,
    pub high_pnl: f64,
    pub low_pnl: f64,
    pub close_pnl: f64,
    pub open_pnl_pct: f64,
    pub mfe_pnl_pct: f64,
    pub mae_pnl_pct: f64,
    pub close_pnl_pct: f64,
    pub mfe_observed_at_ms: i64,
    pub mae_observed_at_ms: i64,
    pub entry_price: f64,
    pub margin: f64,
    pub leverage: f64,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub btc_context_opposed: Option<bool>,
    pub risk_tags: Vec<TradeTag>,
    pub feature_snapshot: Value,
    pub market_snapshot: Value,
}

impl PositionContextEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: &StrategyIdentity,
        run_id: &str,
        position: &PaperPositionSnapshot,
        symbol: &SymbolSnapshot,
        paper: &PaperAccountSnapshot,
        btc: Option<&SymbolSnapshot>,
        event_kind: PositionContextEventKind,
        observed_at_ms: i64,
    ) -> Self {
        let position_key = format!(
            "{}:{run_id}:{}:{}",
            identity.experiment_key(),
            position.inst_id,
            position.opened_at_ms
        );
        let bucket_start_ms = match event_kind {
            PositionContextEventKind::Checkpoint => {
                bucket_start(observed_at_ms, POSITION_CONTEXT_BUCKET_MS)
            }
            PositionContextEventKind::Opened | PositionContextEventKind::ExitSignal => {
                observed_at_ms
            }
        };
        let event_key = match event_kind {
            PositionContextEventKind::Checkpoint => {
                format!("{position_key}:{}:{bucket_start_ms}", event_kind.as_str())
            }
            PositionContextEventKind::Opened | PositionContextEventKind::ExitSignal => {
                format!("{position_key}:{}", event_kind.as_str())
            }
        };
        let current_score = (!position.primary_signal.is_empty())
            .then(|| signal_score_for_primary(symbol, &position.primary_signal));
        let btc_context_opposed = btc.map(|btc| match position.side {
            PaperSide::Long => btc.trend_score.direction == Direction::Short,
            PaperSide::Short => btc.trend_score.direction == Direction::Long,
        });
        Self {
            event_key,
            position_key,
            run_id: run_id.to_string(),
            identity: identity.clone(),
            event_kind,
            bucket_start_ms,
            symbol: position.inst_id.clone(),
            side: position.side,
            primary_signal: position.primary_signal.clone(),
            opened_at_ms: position.opened_at_ms,
            first_observed_at_ms: observed_at_ms,
            last_observed_at_ms: observed_at_ms,
            observation_count: 1,
            position_age_ms: observed_at_ms.saturating_sub(position.opened_at_ms).max(0),
            current_score,
            open_mark_price: position.mark_price,
            high_mark_price: position.mark_price,
            low_mark_price: position.mark_price,
            close_mark_price: position.mark_price,
            open_pnl: position.unrealized_pnl,
            high_pnl: position.unrealized_pnl,
            low_pnl: position.unrealized_pnl,
            close_pnl: position.unrealized_pnl,
            open_pnl_pct: position.pnl_pct,
            mfe_pnl_pct: position.pnl_pct,
            mae_pnl_pct: position.pnl_pct,
            close_pnl_pct: position.pnl_pct,
            mfe_observed_at_ms: observed_at_ms,
            mae_observed_at_ms: observed_at_ms,
            entry_price: position.entry_price,
            margin: position.margin,
            leverage: position.leverage,
            stop_loss: position.stop_loss,
            take_profit: position.take_profit,
            btc_context_opposed,
            risk_tags: position.tags.clone(),
            feature_snapshot: feature_snapshot(symbol),
            market_snapshot: market_snapshot(paper, btc),
        }
    }

    pub fn merge(&mut self, newer: Self) {
        debug_assert_eq!(self.event_key, newer.event_key);
        self.observation_count = self
            .observation_count
            .saturating_add(newer.observation_count);
        if newer.high_pnl > self.high_pnl {
            self.high_pnl = newer.high_pnl;
        }
        if newer.low_pnl < self.low_pnl {
            self.low_pnl = newer.low_pnl;
        }
        if newer.mfe_pnl_pct > self.mfe_pnl_pct {
            self.mfe_pnl_pct = newer.mfe_pnl_pct;
            self.mfe_observed_at_ms = newer.mfe_observed_at_ms;
        }
        if newer.mae_pnl_pct < self.mae_pnl_pct {
            self.mae_pnl_pct = newer.mae_pnl_pct;
            self.mae_observed_at_ms = newer.mae_observed_at_ms;
        }
        self.high_mark_price = self.high_mark_price.max(newer.high_mark_price);
        self.low_mark_price = self.low_mark_price.min(newer.low_mark_price);
        if newer.first_observed_at_ms < self.first_observed_at_ms {
            self.first_observed_at_ms = newer.first_observed_at_ms;
            self.open_mark_price = newer.open_mark_price;
            self.open_pnl = newer.open_pnl;
            self.open_pnl_pct = newer.open_pnl_pct;
        }
        if newer.last_observed_at_ms >= self.last_observed_at_ms {
            self.last_observed_at_ms = newer.last_observed_at_ms;
            self.position_age_ms = newer.position_age_ms;
            self.current_score = newer.current_score;
            self.close_mark_price = newer.close_mark_price;
            self.close_pnl = newer.close_pnl;
            self.close_pnl_pct = newer.close_pnl_pct;
            self.btc_context_opposed = newer.btc_context_opposed;
            self.risk_tags = newer.risk_tags;
            self.feature_snapshot = newer.feature_snapshot;
            self.market_snapshot = newer.market_snapshot;
        }
    }
}

fn bucket_start(timestamp_ms: i64, bucket_size_ms: i64) -> i64 {
    timestamp_ms.div_euclid(bucket_size_ms) * bucket_size_ms
}

fn side_name(side: PaperSide) -> &'static str {
    match side {
        PaperSide::Long => "long",
        PaperSide::Short => "short",
    }
}

fn feature_snapshot(symbol: &SymbolSnapshot) -> Value {
    serde_json::to_value(symbol).expect("symbol snapshot must serialize")
}

fn market_snapshot(paper: &PaperAccountSnapshot, btc: Option<&SymbolSnapshot>) -> Value {
    json!({
        "account": {
            "equity": paper.equity,
            "available_balance": paper.available_balance,
            "used_margin": paper.used_margin,
            "realized_pnl": paper.realized_pnl,
            "unrealized_pnl": paper.unrealized_pnl,
            "open_positions_count": paper.positions.len(),
        },
        "btc": btc.map(|value| json!({
            "price": value.price,
            "change_5m_pct": value.change_5m_pct,
            "change_15m_pct": value.change_15m_pct,
            "change_1h_pct": value.change_1h_pct,
            "trend_score": value.trend_score,
            "range_score": value.range_score,
            "updated_at_ms": value.updated_at_ms,
        })),
    })
}

fn is_session_tag(kind: TradeTagKind) -> bool {
    matches!(
        kind,
        TradeTagKind::TimeRiskAsiaOpen
            | TradeTagKind::TimeRiskMiddayReassessment
            | TradeTagKind::TimeRiskEuUsTransition
            | TradeTagKind::TimeRiskUsData
            | TradeTagKind::TimeRiskUsOpen
            | TradeTagKind::TimeRiskLateUs
    )
}

fn is_event_tag(kind: TradeTagKind) -> bool {
    matches!(
        kind,
        TradeTagKind::TimeRiskWeekdayEvent | TradeTagKind::MiddayReversalWindow
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        auto_strategy::{evaluate_auto_strategy_observed_at, AutoStrategyConfig},
        domain::{ScalpingMetrics, Score},
        paper::{PaperState, SCALPING_OPTIMIZATION_VERSION},
    };
    use std::collections::BTreeMap;

    #[test]
    fn candidate_events_merge_inside_a_fifteen_minute_bucket() {
        let identity = StrategyIdentity::restored_v3();
        let symbol = symbol(90, 1_000);
        let paper = PaperState::fresh_restored_v3(identity.clone())
            .snapshot(&BTreeMap::<String, f64>::new());
        let candidate = evaluate_auto_strategy_observed_at(
            &symbol,
            &paper,
            AutoStrategyConfig::default(),
            1_000,
        )
        .candidate
        .unwrap();
        let mut first = StrategyCandidateEvent::new(
            &identity,
            paper.run_id.as_str(),
            &symbol,
            &paper,
            None,
            candidate.clone(),
            1_000,
        );
        let second = StrategyCandidateEvent::new(
            &identity,
            paper.run_id.as_str(),
            &symbol,
            &paper,
            None,
            candidate,
            20_000,
        );
        let next_bucket = StrategyCandidateEvent::new(
            &identity,
            paper.run_id.as_str(),
            &symbol,
            &paper,
            None,
            evaluate_auto_strategy_observed_at(
                &symbol,
                &paper,
                AutoStrategyConfig::default(),
                CANDIDATE_EVENT_BUCKET_MS + 1_000,
            )
            .candidate
            .unwrap(),
            CANDIDATE_EVENT_BUCKET_MS + 1_000,
        );

        assert_eq!(first.event_key, second.event_key);
        assert_ne!(first.event_key, next_bucket.event_key);
        first.merge(second);
        assert_eq!(first.observation_count, 2);
        assert_eq!(first.first_observed_at_ms, 1_000);
        assert_eq!(first.last_observed_at_ms, 20_000);
        assert_eq!(first.identity.experiment_key(), "v0.1.3/baseline");
    }

    #[test]
    fn position_checkpoints_keep_mfe_and_mae_inside_an_hour_bucket() {
        let identity = StrategyIdentity::restored_v3();
        let mut state = PaperState::fresh_restored_v3(identity.clone());
        let order = crate::paper::PaperOrderRequest::automatic(
            "ETH-USDT-SWAP",
            PaperSide::Long,
            300.0,
            20.0,
            98.5,
            102.0,
            None,
            "trend_long",
            "test",
            Vec::new(),
        );
        state.open(order, 100.0, 10_000.0, 1_000).unwrap();
        let paper_low = state.snapshot(&BTreeMap::from([("ETH-USDT-SWAP".to_string(), 99.0)]));
        let paper_high = state.snapshot(&BTreeMap::from([("ETH-USDT-SWAP".to_string(), 103.0)]));
        let mut low = PositionContextEvent::new(
            &identity,
            paper_low.run_id.as_str(),
            &paper_low.positions[0],
            &symbol(90, 2_000),
            &paper_low,
            None,
            PositionContextEventKind::Checkpoint,
            2_000,
        );
        let high = PositionContextEvent::new(
            &identity,
            paper_high.run_id.as_str(),
            &paper_high.positions[0],
            &symbol(94, 3_000),
            &paper_high,
            None,
            PositionContextEventKind::Checkpoint,
            3_000,
        );

        assert_eq!(low.event_key, high.event_key);
        low.merge(high);
        assert_eq!(low.observation_count, 2);
        assert!(low.mfe_pnl_pct > 0.0);
        assert!(low.mae_pnl_pct < 0.0);
        assert_eq!(low.last_observed_at_ms, 3_000);
    }

    fn symbol(score_value: u8, updated_at_ms: i64) -> SymbolSnapshot {
        SymbolSnapshot {
            inst_id: "ETH-USDT-SWAP".to_string(),
            price: 100.0,
            change_5m_pct: 0.0,
            change_15m_pct: 0.0,
            change_1h_pct: 0.0,
            amplitude_24h_pct: 0.0,
            trend_score: Score {
                value: score_value,
                direction: Direction::Long,
                reasons: Vec::new(),
            },
            range_score: Score {
                value: 0,
                direction: Direction::Neutral,
                reasons: Vec::new(),
            },
            pool_tags: vec!["dynamic".to_string()],
            trigger_reason: SCALPING_OPTIMIZATION_VERSION.to_string(),
            funding_rate: None,
            scalping_metrics: ScalpingMetrics::default(),
            fvgs: Vec::new(),
            levels: Vec::new(),
            pattern_signals: Vec::new(),
            updated_at_ms,
        }
    }
}
