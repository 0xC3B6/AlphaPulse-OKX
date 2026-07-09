use std::collections::{BTreeMap, BTreeSet, VecDeque};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::{
    domain::{Direction, SymbolSnapshot},
    paper::{
        PaperAccountSnapshot, PaperError, PaperOrderRequest, PaperSide, PaperState, PaperTrade,
        PaperTradeAction,
    },
};

pub const V3_VERSION_CODE: &str = "v0.1.3";
pub const V4_VERSION_CODE: &str = "v0.1.4";

const MAX_RISK_EVENTS: usize = 300;
const MAX_EQUITY_SNAPSHOTS: usize = 1_000;
const DEFAULT_MARGIN: f64 = 100.0;
const DEFAULT_LEVERAGE: f64 = 10.0;
const AUTO_OPEN_SCORE: u8 = 80;
const ACCOUNT_KILL_SWITCH_DRAWDOWN: f64 = 0.30;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyVersionStatus {
    Active,
    Testing,
    Archived,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyRunMode {
    Paper,
    Shadow,
    Live,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyRunStatus {
    Running,
    Stopped,
    Reset,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyVersion {
    pub version_code: String,
    pub name: String,
    pub description: String,
    pub status: StrategyVersionStatus,
    pub config_json: serde_json::Value,
    pub config_hash: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRun {
    pub run_id: String,
    pub version_code: String,
    pub mode: StrategyRunMode,
    pub status: StrategyRunStatus,
    pub initial_equity: f64,
    pub current_equity: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub fee_total: f64,
    pub max_drawdown: f64,
    pub start_time_ms: i64,
    pub end_time_ms: Option<i64>,
    pub fee_model: String,
    pub slippage_model: String,
    pub config_snapshot: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderAction {
    Open,
    Close,
    Reduce,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderIntent {
    pub version_code: String,
    pub run_id: String,
    pub symbol: String,
    pub side: PaperSide,
    pub action: OrderAction,
    pub margin: f64,
    pub leverage: f64,
    pub score: u8,
    pub primary_signal: String,
    pub reason: String,
    pub tags: Vec<String>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub expire_at: Option<i64>,
    pub risk_flags: Vec<String>,
    pub risk_guard_decision: Option<String>,
    pub config_hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketRiskSnapshot {
    pub symbol: String,
    pub change_24h_pct: Option<f64>,
    pub change_48h_pct: Option<f64>,
    pub change_72h_pct: Option<f64>,
    pub intraday_low_break_count: usize,
    pub high_volatility_flag: bool,
    pub consecutive_dump_days: usize,
}

impl MarketRiskSnapshot {
    pub fn for_symbol(symbol: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            ..Self::default()
        }
    }

    fn from_symbol(symbol: &SymbolSnapshot) -> Self {
        let consecutive_dump_days = consecutive_dump_days(symbol);
        Self {
            symbol: symbol.inst_id.clone(),
            change_24h_pct: symbol.change_24h_pct,
            change_48h_pct: symbol.change_48h_pct,
            change_72h_pct: symbol.change_72h_pct,
            intraday_low_break_count: symbol.intraday_low_break_count,
            high_volatility_flag: symbol.high_volatility_flag,
            consecutive_dump_days,
        }
    }

    fn is_extreme_dump(&self) -> bool {
        self.change_24h_pct.is_some_and(|change| change <= -0.30)
            || self.change_48h_pct.is_some_and(|change| change <= -0.50)
            || self.change_72h_pct.is_some_and(|change| change <= -0.60)
            || self.consecutive_dump_days >= 2
    }
}

fn consecutive_dump_days(symbol: &SymbolSnapshot) -> usize {
    let mut count = 0;
    if symbol.change_24h_pct.is_some_and(|change| change <= -0.25) {
        count += 1;
    }
    if let (Some(change_24h), Some(change_48h)) = (symbol.change_24h_pct, symbol.change_48h_pct) {
        if change_48h - change_24h <= -0.25 {
            count += 1;
        }
    }
    count
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAccountContext {
    pub same_symbol_unrealized_pnl_pct: Option<f64>,
    pub account_equity: f64,
    pub account_peak_equity: f64,
}

impl Default for RiskAccountContext {
    fn default() -> Self {
        Self {
            same_symbol_unrealized_pnl_pct: None,
            account_equity: 10_000.0,
            account_peak_equity: 10_000.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskGuardDecision {
    Allowed,
    Blocked,
    Reduced,
    LeverageCapped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskGuardOutcome {
    pub decision: RiskGuardDecision,
    pub final_intent: Option<OrderIntent>,
    pub event: Option<RiskGuardEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopLossRecord {
    pub version_code: String,
    pub run_id: String,
    pub symbol: String,
    pub side: PaperSide,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskGuardEvent {
    pub id: u64,
    pub run_id: String,
    pub version_code: String,
    pub timestamp_ms: i64,
    pub symbol: String,
    pub side: PaperSide,
    pub original_signal: String,
    pub score: u8,
    pub action: String,
    pub reason: String,
    pub risk_flags: Vec<String>,
    pub original_order_intent: OrderIntent,
    pub final_order_intent: Option<OrderIntent>,
}

#[derive(Debug, Clone)]
pub struct RiskGuard {
    cooldown_ms: i64,
    stop_losses: VecDeque<StopLossRecord>,
    quarantined_symbols: BTreeSet<String>,
}

impl Default for RiskGuard {
    fn default() -> Self {
        Self {
            cooldown_ms: 2 * 60 * 60 * 1_000,
            stop_losses: VecDeque::new(),
            quarantined_symbols: ["LAB-USDT-SWAP", "LAB", "PUMP-USDT-SWAP", "PUMP"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

impl RiskGuard {
    pub fn record_stop_loss(&mut self, record: StopLossRecord) {
        self.stop_losses.push_back(record);
        while self.stop_losses.len() > 500 {
            self.stop_losses.pop_front();
        }
    }

    pub fn evaluate(
        &self,
        intent: &OrderIntent,
        market: &MarketRiskSnapshot,
        account: &RiskAccountContext,
        now_ms: i64,
    ) -> RiskGuardOutcome {
        if intent.action != OrderAction::Open {
            return RiskGuardOutcome {
                decision: RiskGuardDecision::Allowed,
                final_intent: Some(intent.clone()),
                event: None,
            };
        }

        if account.account_peak_equity > 0.0
            && account.account_equity / account.account_peak_equity
                <= 1.0 - ACCOUNT_KILL_SWITCH_DRAWDOWN
        {
            return self.block(
                intent,
                now_ms,
                "blocked_by_account_kill_switch",
                &["account_kill_switch"],
            );
        }

        if account
            .same_symbol_unrealized_pnl_pct
            .is_some_and(|pnl_pct| pnl_pct < -0.03)
        {
            return self.block(
                intent,
                now_ms,
                "blocked_by_no_add_to_loser",
                &["no_add_to_loser"],
            );
        }

        if self.is_in_cooldown(intent, now_ms) {
            return self.block(
                intent,
                now_ms,
                "blocked_by_same_symbol_cooldown",
                &["same_symbol_cooldown"],
            );
        }

        let mut final_intent = intent.clone();
        let mut action = RiskGuardDecision::Allowed;
        let mut flags = Vec::new();

        let extreme_dump = market.is_extreme_dump();
        if extreme_dump && intent.side == PaperSide::Long && is_normal_long(intent) {
            return self.block(
                intent,
                now_ms,
                "blocked_by_extreme_dump_gate",
                &["extreme_dump", "ban_normal_long"],
            );
        }

        if self.is_quarantined(&intent.symbol) {
            flags.push("symbol_quarantine".to_string());
            if extreme_dump {
                return self.block(
                    intent,
                    now_ms,
                    "symbol_quarantine",
                    &["symbol_quarantine", "extreme_dump"],
                );
            }
            if intent.side == PaperSide::Long && is_normal_long(intent) {
                return self.block(intent, now_ms, "symbol_quarantine", &["symbol_quarantine"]);
            }
            final_intent.margin *= 0.25;
            flags.push("size_reduced_by_symbol_quarantine".to_string());
            action = RiskGuardDecision::Reduced;
            if final_intent.leverage > 3.0 {
                final_intent.leverage = 3.0;
                flags.push("leverage_capped_by_symbol_quarantine".to_string());
                action = RiskGuardDecision::LeverageCapped;
            }
        }

        if intent.score <= 85 {
            return self.block(
                intent,
                now_ms,
                "blocked_by_score_80_85_shadow_only",
                &["score_80_85"],
            );
        }

        let risky_tags = risky_tag_flags(intent);
        if !risky_tags.is_empty() {
            for flag in risky_tags {
                push_unique(&mut flags, &flag);
            }
            if intent.tags.iter().any(|tag| is_time_risk_tag(tag))
                && (intent.side == PaperSide::Long
                    || intent.tags.iter().any(|tag| tag == "pattern"))
            {
                return self.block(intent, now_ms, "blocked_by_time_risk", &["time_risk"]);
            }
            if intent.tags.iter().any(|tag| tag == "pattern") {
                final_intent.margin *= 0.5;
                action = RiskGuardDecision::Reduced;
            }
            if intent
                .tags
                .iter()
                .any(|tag| tag == "requires_high_confidence")
            {
                final_intent.margin *= 0.7;
                action = RiskGuardDecision::Reduced;
            }
        }

        final_intent.risk_flags.extend(flags.clone());
        final_intent.risk_guard_decision = Some(format!("{action:?}").to_lowercase());
        if action == RiskGuardDecision::Allowed {
            RiskGuardOutcome {
                decision: RiskGuardDecision::Allowed,
                final_intent: Some(final_intent),
                event: None,
            }
        } else {
            RiskGuardOutcome {
                decision: action.clone(),
                final_intent: Some(final_intent.clone()),
                event: Some(self.event(
                    intent,
                    now_ms,
                    format!("{:?}", action).to_lowercase(),
                    "risk_guard_adjusted",
                    flags,
                    Some(final_intent),
                )),
            }
        }
    }

    fn is_in_cooldown(&self, intent: &OrderIntent, now_ms: i64) -> bool {
        self.stop_losses.iter().rev().any(|record| {
            record.version_code == intent.version_code
                && record.run_id == intent.run_id
                && record.symbol == intent.symbol
                && record.side == intent.side
                && now_ms >= record.ts_ms
                && now_ms - record.ts_ms <= self.cooldown_ms
        })
    }

    fn is_quarantined(&self, symbol: &str) -> bool {
        self.quarantined_symbols.contains(symbol)
            || self
                .quarantined_symbols
                .contains(symbol.trim_end_matches("-USDT-SWAP"))
    }

    fn block(
        &self,
        intent: &OrderIntent,
        now_ms: i64,
        reason: &str,
        flags: &[&str],
    ) -> RiskGuardOutcome {
        let flags = flags
            .iter()
            .map(|flag| (*flag).to_string())
            .collect::<Vec<_>>();
        RiskGuardOutcome {
            decision: RiskGuardDecision::Blocked,
            final_intent: None,
            event: Some(self.event(intent, now_ms, "blocked".to_string(), reason, flags, None)),
        }
    }

    fn event(
        &self,
        intent: &OrderIntent,
        now_ms: i64,
        action: String,
        reason: &str,
        risk_flags: Vec<String>,
        final_order_intent: Option<OrderIntent>,
    ) -> RiskGuardEvent {
        RiskGuardEvent {
            id: now_ms.max(0) as u64,
            run_id: intent.run_id.clone(),
            version_code: intent.version_code.clone(),
            timestamp_ms: now_ms,
            symbol: intent.symbol.clone(),
            side: intent.side,
            original_signal: intent.primary_signal.clone(),
            score: intent.score,
            action,
            reason: reason.to_string(),
            risk_flags,
            original_order_intent: intent.clone(),
            final_order_intent,
        }
    }
}

fn is_normal_long(intent: &OrderIntent) -> bool {
    if intent.primary_signal == "sweep_failure_long" {
        return false;
    }
    intent.primary_signal.contains("long")
        || intent.tags.iter().any(|tag| {
            matches!(
                tag.as_str(),
                "range_long" | "pattern_long" | "mover_24h_long" | "long"
            )
        })
}

fn risky_tag_flags(intent: &OrderIntent) -> Vec<String> {
    let mut flags = Vec::new();
    for tag in &intent.tags {
        match tag.as_str() {
            "pattern" | "pattern_long" | "pattern_short" => {
                push_unique(&mut flags, "pattern_downgrade")
            }
            "range_long" => push_unique(&mut flags, "range_long_downgrade"),
            "requires_high_confidence" => push_unique(&mut flags, "requires_high_confidence"),
            tag if is_time_risk_tag(tag) => push_unique(&mut flags, "time_risk"),
            _ => {}
        }
    }
    flags
}

fn is_time_risk_tag(tag: &str) -> bool {
    tag == "time_risk_us_open" || tag == "time_penalty_18" || tag == "us_open"
}

fn push_unique(flags: &mut Vec<String>, flag: &str) {
    if !flags.iter().any(|existing| existing == flag) {
        flags.push(flag.to_string());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionConfidence {
    InsufficientSample,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionSuggestion {
    InsufficientSample,
    Quality,
    Keep,
    Observe,
    Fragile,
    Downgrade,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionTrade {
    pub id: u64,
    pub symbol: String,
    pub side: PaperSide,
    pub primary_signal: String,
    pub tags: Vec<String>,
    pub net_pnl: f64,
    pub exit_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributionRow {
    pub key: String,
    pub sample_count: usize,
    pub profit_factor: Option<f64>,
    pub net_pnl: f64,
    pub win_rate: Option<f64>,
    pub avg_pnl: Option<f64>,
    pub avg_win: Option<f64>,
    pub avg_loss: Option<f64>,
    pub max_loss: Option<f64>,
    pub stop_loss_rate: Option<f64>,
    pub take_profit_rate: Option<f64>,
    pub confidence: AttributionConfidence,
    pub suggestion: AttributionSuggestion,
}

pub fn attribution_by_signal(trades: &[AttributionTrade]) -> Vec<AttributionRow> {
    attribution_by_key(trades, |trade| vec![trade.primary_signal.clone()])
}

pub fn attribution_by_tag(trades: &[AttributionTrade]) -> Vec<AttributionRow> {
    attribution_by_key(trades, |trade| trade.tags.clone())
}

pub fn attribution_by_combo(trades: &[AttributionTrade]) -> Vec<AttributionRow> {
    attribution_by_key(trades, |trade| {
        let mut tags = trade.tags.clone();
        tags.sort();
        let mut combos = Vec::new();
        for left in 0..tags.len() {
            for right in (left + 1)..tags.len() {
                combos.push(format!("{} + {}", tags[left], tags[right]));
            }
        }
        combos
    })
}

pub fn attribution_by_symbol(trades: &[AttributionTrade]) -> Vec<AttributionRow> {
    attribution_by_key(trades, |trade| vec![trade.symbol.clone()])
}

fn attribution_by_key(
    trades: &[AttributionTrade],
    keys: impl Fn(&AttributionTrade) -> Vec<String>,
) -> Vec<AttributionRow> {
    let mut groups: BTreeMap<String, Vec<AttributionTrade>> = BTreeMap::new();
    for trade in trades {
        for key in keys(trade) {
            if key.trim().is_empty() {
                continue;
            }
            groups.entry(key).or_default().push(trade.clone());
        }
    }

    let mut rows = groups
        .into_iter()
        .map(|(key, group)| attribution_row(key, &group))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .net_pnl
            .partial_cmp(&left.net_pnl)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.sample_count.cmp(&left.sample_count))
    });
    rows
}

fn attribution_row(key: String, trades: &[AttributionTrade]) -> AttributionRow {
    let sample_count = trades.len();
    let winners = trades
        .iter()
        .filter(|trade| trade.net_pnl > 0.0)
        .collect::<Vec<_>>();
    let losers = trades
        .iter()
        .filter(|trade| trade.net_pnl < 0.0)
        .collect::<Vec<_>>();
    let gross_profit = winners.iter().map(|trade| trade.net_pnl).sum::<f64>();
    let gross_loss_abs = losers.iter().map(|trade| trade.net_pnl).sum::<f64>().abs();
    let net_pnl = trades.iter().map(|trade| trade.net_pnl).sum::<f64>();
    let profit_factor = if gross_profit > 0.0 && gross_loss_abs > 0.0 {
        Some(gross_profit / gross_loss_abs)
    } else {
        None
    };
    let win_rate = ratio(winners.len(), sample_count);
    let stop_loss_count = trades
        .iter()
        .filter(|trade| trade.exit_reason.contains("stop_loss"))
        .count();
    let take_profit_count = trades
        .iter()
        .filter(|trade| trade.exit_reason.contains("take_profit"))
        .count();
    let stop_loss_rate = ratio(stop_loss_count, sample_count);
    let take_profit_rate = ratio(take_profit_count, sample_count);
    let max_loss = losers
        .iter()
        .map(|trade| trade.net_pnl)
        .min_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let confidence = confidence_for_sample(sample_count);
    let suggestion = suggestion_for_stats(sample_count, profit_factor, max_loss, stop_loss_rate);

    AttributionRow {
        key,
        sample_count,
        profit_factor,
        net_pnl,
        win_rate,
        avg_pnl: average(trades.iter().map(|trade| trade.net_pnl)),
        avg_win: average(winners.iter().map(|trade| trade.net_pnl)),
        avg_loss: average(losers.iter().map(|trade| trade.net_pnl)),
        max_loss,
        stop_loss_rate,
        take_profit_rate,
        confidence,
        suggestion,
    }
}

fn confidence_for_sample(sample_count: usize) -> AttributionConfidence {
    match sample_count {
        0..=9 => AttributionConfidence::InsufficientSample,
        10..=29 => AttributionConfidence::Low,
        30..=99 => AttributionConfidence::Medium,
        _ => AttributionConfidence::High,
    }
}

fn suggestion_for_stats(
    sample_count: usize,
    profit_factor: Option<f64>,
    max_loss: Option<f64>,
    stop_loss_rate: Option<f64>,
) -> AttributionSuggestion {
    if sample_count < 10 {
        return AttributionSuggestion::InsufficientSample;
    }
    if stop_loss_rate.is_some_and(|rate| rate > 0.60) || max_loss.is_some_and(|loss| loss <= -100.0)
    {
        return AttributionSuggestion::Downgrade;
    }
    if sample_count >= 30 && profit_factor.is_some_and(|pf| pf >= 1.6) {
        return AttributionSuggestion::Quality;
    }
    if sample_count >= 30 && profit_factor.is_some_and(|pf| pf >= 1.3) {
        return AttributionSuggestion::Keep;
    }
    if sample_count >= 30 && profit_factor.is_some_and(|pf| pf >= 1.1) {
        return AttributionSuggestion::Observe;
    }
    if sample_count >= 30 && profit_factor.is_some_and(|pf| pf >= 1.0) {
        return AttributionSuggestion::Fragile;
    }
    AttributionSuggestion::Downgrade
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

#[derive(Debug, Clone, Serialize)]
pub struct StrategyEquitySnapshot {
    pub run_id: String,
    pub version_code: String,
    pub timestamp_ms: i64,
    pub equity: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub drawdown: f64,
    pub open_positions_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyVersionOverview {
    pub version_code: String,
    pub name: String,
    pub status: StrategyVersionStatus,
    pub mode: StrategyRunMode,
    pub run_id: String,
    pub run_time_ms: i64,
    pub initial_equity: f64,
    pub current_equity: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub return_pct: f64,
    pub max_drawdown: f64,
    pub win_rate: Option<f64>,
    pub profit_factor: Option<f64>,
    pub closed_trades: usize,
    pub open_positions: usize,
    pub total_fee: f64,
    pub config_hash: String,
    pub last_updated_ms: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyVersionSnapshot {
    pub version: StrategyVersion,
    pub run: StrategyRun,
    pub overview: StrategyVersionOverview,
    pub paper: PaperAccountSnapshot,
    pub equity: Vec<StrategyEquitySnapshot>,
    pub signal_attribution: Vec<AttributionRow>,
    pub tag_attribution: Vec<AttributionRow>,
    pub combo_attribution: Vec<AttributionRow>,
    pub symbol_attribution: Vec<AttributionRow>,
    pub risk_guard_events: Vec<RiskGuardEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StrategyCenterSnapshot {
    pub versions: Vec<StrategyVersionSnapshot>,
    pub last_updated_ms: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum StrategyError {
    #[error("unknown strategy version {0}")]
    UnknownVersion(String),
    #[error(transparent)]
    Paper(#[from] PaperError),
}

#[derive(Debug, Clone)]
struct VersionPaperAccount {
    version: StrategyVersion,
    run: StrategyRun,
    paper: PaperState,
    equity_snapshots: VecDeque<StrategyEquitySnapshot>,
    risk_guard_events: VecDeque<RiskGuardEvent>,
    peak_equity: f64,
    max_drawdown: f64,
}

#[derive(Debug, Clone)]
pub struct VersionedPaperState {
    accounts: BTreeMap<String, VersionPaperAccount>,
    run_sequences: BTreeMap<String, u64>,
    risk_guard: RiskGuard,
    next_risk_event_id: u64,
}

impl Default for VersionedPaperState {
    fn default() -> Self {
        let now_ms = Utc::now().timestamp_millis();
        let mut accounts = BTreeMap::new();
        accounts.insert(
            V3_VERSION_CODE.to_string(),
            VersionPaperAccount::new(v3_version(now_ms), 1, now_ms),
        );
        accounts.insert(
            V4_VERSION_CODE.to_string(),
            VersionPaperAccount::new(v4_version(now_ms), 1, now_ms),
        );
        Self {
            accounts,
            run_sequences: BTreeMap::from([
                (V3_VERSION_CODE.to_string(), 1),
                (V4_VERSION_CODE.to_string(), 1),
            ]),
            risk_guard: RiskGuard::default(),
            next_risk_event_id: 1,
        }
    }
}

impl VersionedPaperState {
    pub fn default_paper_snapshot(
        &self,
        prices: &BTreeMap<String, SymbolSnapshot>,
    ) -> PaperAccountSnapshot {
        self.accounts
            .get(V3_VERSION_CODE)
            .map(|account| account.paper.snapshot(prices))
            .unwrap_or_default()
    }

    pub fn center_snapshot(
        &self,
        prices: &BTreeMap<String, SymbolSnapshot>,
    ) -> StrategyCenterSnapshot {
        let last_updated_ms = Utc::now().timestamp_millis();
        StrategyCenterSnapshot {
            versions: self
                .accounts
                .values()
                .map(|account| account.snapshot(prices, last_updated_ms))
                .collect(),
            last_updated_ms,
        }
    }

    pub fn version_snapshot(
        &self,
        version_code: &str,
        prices: &BTreeMap<String, SymbolSnapshot>,
    ) -> Result<StrategyVersionSnapshot, StrategyError> {
        self.accounts
            .get(version_code)
            .map(|account| account.snapshot(prices, Utc::now().timestamp_millis()))
            .ok_or_else(|| StrategyError::UnknownVersion(version_code.to_string()))
    }

    pub fn open_order(
        &mut self,
        version_code: &str,
        mut request: PaperOrderRequest,
        prices: &BTreeMap<String, SymbolSnapshot>,
        ts_ms: i64,
    ) -> Result<StrategyVersionSnapshot, StrategyError> {
        let account = self
            .accounts
            .get_mut(version_code)
            .ok_or_else(|| StrategyError::UnknownVersion(version_code.to_string()))?;
        prepare_order_metadata(&mut request, account, "manual");
        let price = prices
            .get(&request.inst_id)
            .map(|symbol| symbol.price)
            .filter(|price| *price > 0.0 && price.is_finite())
            .ok_or_else(|| PaperError::PriceUnavailable(request.inst_id.clone()))?;
        let available_balance = account.paper.snapshot(prices).available_balance;
        account
            .paper
            .open(request, price, available_balance, ts_ms)?;
        account.record_equity(prices, ts_ms);
        Ok(account.snapshot(prices, ts_ms))
    }

    pub fn close_position(
        &mut self,
        version_code: &str,
        inst_id: &str,
        prices: &BTreeMap<String, SymbolSnapshot>,
        ts_ms: i64,
    ) -> Result<StrategyVersionSnapshot, StrategyError> {
        let account = self
            .accounts
            .get_mut(version_code)
            .ok_or_else(|| StrategyError::UnknownVersion(version_code.to_string()))?;
        let price = prices
            .get(inst_id)
            .map(|symbol| symbol.price)
            .filter(|price| *price > 0.0 && price.is_finite())
            .ok_or_else(|| PaperError::PriceUnavailable(inst_id.to_string()))?;
        let trade = account.paper.close(inst_id, price, ts_ms)?;
        if trade.realized_pnl < 0.0 {
            self.risk_guard.record_stop_loss(StopLossRecord {
                version_code: trade.version_code.clone(),
                run_id: trade.run_id.clone(),
                symbol: trade.inst_id.clone(),
                side: trade.side,
                ts_ms,
            });
        }
        account.record_equity(prices, ts_ms);
        Ok(account.snapshot(prices, ts_ms))
    }

    pub fn reset_version(&mut self, version_code: &str, ts_ms: i64) -> Result<(), StrategyError> {
        let next_sequence = self
            .run_sequences
            .entry(version_code.to_string())
            .or_insert(1);
        *next_sequence += 1;
        let account = self
            .accounts
            .get_mut(version_code)
            .ok_or_else(|| StrategyError::UnknownVersion(version_code.to_string()))?;
        let version = account.version.clone();
        *account = VersionPaperAccount::new(version, *next_sequence, ts_ms);
        Ok(())
    }

    pub fn start_version(&mut self, version_code: &str, ts_ms: i64) -> Result<(), StrategyError> {
        let account = self
            .accounts
            .get_mut(version_code)
            .ok_or_else(|| StrategyError::UnknownVersion(version_code.to_string()))?;
        account.run.status = StrategyRunStatus::Running;
        account.run.end_time_ms = None;
        account.version.status = if version_code == V4_VERSION_CODE {
            StrategyVersionStatus::Testing
        } else {
            StrategyVersionStatus::Active
        };
        account.version.updated_at_ms = ts_ms;
        Ok(())
    }

    pub fn stop_version(&mut self, version_code: &str, ts_ms: i64) -> Result<(), StrategyError> {
        let account = self
            .accounts
            .get_mut(version_code)
            .ok_or_else(|| StrategyError::UnknownVersion(version_code.to_string()))?;
        account.run.status = StrategyRunStatus::Stopped;
        account.run.end_time_ms = Some(ts_ms);
        account.version.updated_at_ms = ts_ms;
        Ok(())
    }

    pub fn process_market_update(
        &mut self,
        symbol: &SymbolSnapshot,
        prices: &BTreeMap<String, SymbolSnapshot>,
        ts_ms: i64,
    ) {
        let Some(base_intent) = self.build_base_intent(symbol, V3_VERSION_CODE) else {
            return;
        };
        let version_codes = self.accounts.keys().cloned().collect::<Vec<_>>();
        for version_code in version_codes {
            let Some(account) = self.accounts.get(&version_code) else {
                continue;
            };
            if account.run.status != StrategyRunStatus::Running
                || account.paper.has_position(&symbol.inst_id)
            {
                continue;
            }
            let mut intent = base_intent.clone();
            if let Some(account) = self.accounts.get(&version_code) {
                intent.version_code = version_code.clone();
                intent.run_id = account.run.run_id.clone();
                intent.config_hash = account.version.config_hash.clone();
            }
            if version_code == V4_VERSION_CODE {
                let account_context = self.account_context(&version_code, &intent, prices);
                let outcome = self.risk_guard.evaluate(
                    &intent,
                    &MarketRiskSnapshot::from_symbol(symbol),
                    &account_context,
                    ts_ms,
                );
                if let Some(mut event) = outcome.event {
                    event.id = self.next_event_id();
                    self.record_risk_event(&version_code, event);
                }
                if let Some(final_intent) = outcome.final_intent {
                    let _ = self.execute_intent(final_intent, prices, ts_ms);
                }
            } else {
                let _ = self.execute_intent(intent, prices, ts_ms);
            }
        }
    }

    fn build_base_intent(
        &self,
        symbol: &SymbolSnapshot,
        version_code: &str,
    ) -> Option<OrderIntent> {
        let (kind, score) = if symbol.trend_score.value >= symbol.range_score.value {
            ("trend", &symbol.trend_score)
        } else {
            ("range", &symbol.range_score)
        };
        if score.direction == Direction::Neutral || score.value < AUTO_OPEN_SCORE {
            return None;
        }
        let side = match score.direction {
            Direction::Long => PaperSide::Long,
            Direction::Short => PaperSide::Short,
            Direction::Neutral => return None,
        };
        let primary_signal = format!(
            "{}_{}",
            kind,
            match side {
                PaperSide::Long => "long",
                PaperSide::Short => "short",
            }
        );
        let mut tags = vec![
            match side {
                PaperSide::Long => "long".to_string(),
                PaperSide::Short => "short".to_string(),
            },
            primary_signal.clone(),
            score_bucket(score.value),
        ];
        tags.extend(symbol.pool_tags.clone());
        if kind == "range" {
            tags.push("range".to_string());
        }
        if score
            .reasons
            .iter()
            .any(|reason| reason.to_lowercase().contains("fvg"))
        {
            tags.push("pattern".to_string());
        }
        let run_id = self
            .accounts
            .get(version_code)
            .map(|account| account.run.run_id.clone())
            .unwrap_or_else(|| format!("{version_code}-paper-1"));
        let config_hash = self
            .accounts
            .get(version_code)
            .map(|account| account.version.config_hash.clone())
            .unwrap_or_else(|| "unknown".to_string());
        Some(OrderIntent {
            version_code: version_code.to_string(),
            run_id,
            symbol: symbol.inst_id.clone(),
            side,
            action: OrderAction::Open,
            margin: DEFAULT_MARGIN,
            leverage: DEFAULT_LEVERAGE,
            score: score.value,
            primary_signal,
            reason: symbol.trigger_reason.clone(),
            tags,
            stop_loss: None,
            take_profit: None,
            expire_at: None,
            risk_flags: Vec::new(),
            risk_guard_decision: None,
            config_hash,
        })
    }

    fn execute_intent(
        &mut self,
        intent: OrderIntent,
        prices: &BTreeMap<String, SymbolSnapshot>,
        ts_ms: i64,
    ) -> Result<PaperTrade, StrategyError> {
        let account = self
            .accounts
            .get_mut(&intent.version_code)
            .ok_or_else(|| StrategyError::UnknownVersion(intent.version_code.clone()))?;
        let price = prices
            .get(&intent.symbol)
            .map(|symbol| symbol.price)
            .filter(|price| *price > 0.0 && price.is_finite())
            .ok_or_else(|| PaperError::PriceUnavailable(intent.symbol.clone()))?;
        let available_balance = account.paper.snapshot(prices).available_balance;
        let trade = account.paper.open(
            PaperOrderRequest::from_intent(&intent),
            price,
            available_balance,
            ts_ms,
        )?;
        account.record_equity(prices, ts_ms);
        Ok(trade)
    }

    fn account_context(
        &self,
        version_code: &str,
        intent: &OrderIntent,
        prices: &BTreeMap<String, SymbolSnapshot>,
    ) -> RiskAccountContext {
        let Some(account) = self.accounts.get(version_code) else {
            return RiskAccountContext::default();
        };
        let paper = account.paper.snapshot(prices);
        RiskAccountContext {
            same_symbol_unrealized_pnl_pct: account.paper.position_pnl_pct(
                &intent.symbol,
                intent.side,
                prices,
            ),
            account_equity: paper.equity,
            account_peak_equity: account.peak_equity.max(paper.equity),
        }
    }

    fn record_risk_event(&mut self, version_code: &str, event: RiskGuardEvent) {
        if let Some(account) = self.accounts.get_mut(version_code) {
            account.risk_guard_events.push_back(event);
            while account.risk_guard_events.len() > MAX_RISK_EVENTS {
                account.risk_guard_events.pop_front();
            }
        }
    }

    fn next_event_id(&mut self) -> u64 {
        let id = self.next_risk_event_id;
        self.next_risk_event_id += 1;
        id
    }
}

impl VersionPaperAccount {
    fn new(version: StrategyVersion, sequence: u64, ts_ms: i64) -> Self {
        let run_id = format!("{}-paper-{sequence}", version.version_code);
        let run = StrategyRun {
            run_id,
            version_code: version.version_code.clone(),
            mode: StrategyRunMode::Paper,
            status: if version.status == StrategyVersionStatus::Disabled {
                StrategyRunStatus::Stopped
            } else {
                StrategyRunStatus::Running
            },
            initial_equity: 10_000.0,
            current_equity: 10_000.0,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
            fee_total: 0.0,
            max_drawdown: 0.0,
            start_time_ms: ts_ms,
            end_time_ms: None,
            fee_model: "paper_zero_fee".to_string(),
            slippage_model: "paper_mark_price".to_string(),
            config_snapshot: version.config_json.clone(),
        };
        Self {
            version,
            run,
            paper: PaperState::default(),
            equity_snapshots: VecDeque::new(),
            risk_guard_events: VecDeque::new(),
            peak_equity: 10_000.0,
            max_drawdown: 0.0,
        }
    }

    fn record_equity(&mut self, prices: &BTreeMap<String, SymbolSnapshot>, ts_ms: i64) {
        let paper = self.paper.snapshot(prices);
        self.peak_equity = self.peak_equity.max(paper.equity);
        let drawdown = if self.peak_equity > 0.0 {
            paper.equity / self.peak_equity - 1.0
        } else {
            0.0
        };
        self.max_drawdown = self.max_drawdown.min(drawdown);
        self.run.current_equity = paper.equity;
        self.run.realized_pnl = paper.realized_pnl;
        self.run.unrealized_pnl = paper.unrealized_pnl;
        self.run.fee_total = paper.total_fees;
        self.run.max_drawdown = self.max_drawdown;
        self.equity_snapshots.push_back(StrategyEquitySnapshot {
            run_id: self.run.run_id.clone(),
            version_code: self.version.version_code.clone(),
            timestamp_ms: ts_ms,
            equity: paper.equity,
            realized_pnl: paper.realized_pnl,
            unrealized_pnl: paper.unrealized_pnl,
            drawdown,
            open_positions_count: paper.positions.len(),
        });
        while self.equity_snapshots.len() > MAX_EQUITY_SNAPSHOTS {
            self.equity_snapshots.pop_front();
        }
    }

    fn snapshot(
        &self,
        prices: &BTreeMap<String, SymbolSnapshot>,
        now_ms: i64,
    ) -> StrategyVersionSnapshot {
        let mut account = self.clone();
        account.record_equity(prices, now_ms);
        let paper = account.paper.snapshot(prices);
        let attribution_trades = paper
            .position_history
            .iter()
            .map(AttributionTrade::from_closed_position)
            .collect::<Vec<_>>();
        StrategyVersionSnapshot {
            version: account.version.clone(),
            run: account.run.clone(),
            overview: account.overview(&paper, now_ms),
            paper,
            equity: account.equity_snapshots.iter().cloned().collect(),
            signal_attribution: attribution_by_signal(&attribution_trades),
            tag_attribution: attribution_by_tag(&attribution_trades),
            combo_attribution: attribution_by_combo(&attribution_trades),
            symbol_attribution: attribution_by_symbol(&attribution_trades),
            risk_guard_events: account.risk_guard_events.iter().rev().cloned().collect(),
        }
    }

    fn overview(&self, paper: &PaperAccountSnapshot, now_ms: i64) -> StrategyVersionOverview {
        StrategyVersionOverview {
            version_code: self.version.version_code.clone(),
            name: self.version.name.clone(),
            status: self.version.status.clone(),
            mode: self.run.mode.clone(),
            run_id: self.run.run_id.clone(),
            run_time_ms: now_ms.saturating_sub(self.run.start_time_ms),
            initial_equity: paper.initial_balance,
            current_equity: paper.equity,
            realized_pnl: paper.realized_pnl,
            unrealized_pnl: paper.unrealized_pnl,
            return_pct: if paper.initial_balance > 0.0 {
                paper.equity / paper.initial_balance - 1.0
            } else {
                0.0
            },
            max_drawdown: self.max_drawdown,
            win_rate: paper.win_rate,
            profit_factor: paper.profit_factor,
            closed_trades: paper.closed_position_count,
            open_positions: paper.positions.len(),
            total_fee: paper.total_fees,
            config_hash: self.version.config_hash.clone(),
            last_updated_ms: now_ms,
        }
    }
}

impl AttributionTrade {
    fn from_closed_position(position: &crate::paper::PaperClosedPositionSnapshot) -> Self {
        Self {
            id: position.id,
            symbol: position.inst_id.clone(),
            side: position.side,
            primary_signal: position.primary_signal.clone(),
            tags: position.tags.clone(),
            net_pnl: position.realized_pnl,
            exit_reason: position.close_reason.clone(),
        }
    }
}

impl PaperOrderRequest {
    pub fn from_intent(intent: &OrderIntent) -> Self {
        Self {
            inst_id: intent.symbol.clone(),
            side: intent.side,
            margin: intent.margin,
            leverage: intent.leverage,
            version_code: Some(intent.version_code.clone()),
            run_id: Some(intent.run_id.clone()),
            primary_signal: Some(intent.primary_signal.clone()),
            tags: intent.tags.clone(),
            risk_flags: intent.risk_flags.clone(),
            risk_guard_decision: intent.risk_guard_decision.clone(),
            strategy_reason: Some(intent.reason.clone()),
            config_hash: Some(intent.config_hash.clone()),
        }
    }
}

fn prepare_order_metadata(
    request: &mut PaperOrderRequest,
    account: &VersionPaperAccount,
    fallback_signal: &str,
) {
    request.version_code = Some(account.version.version_code.clone());
    request.run_id = Some(account.run.run_id.clone());
    if request.primary_signal.is_none() {
        request.primary_signal = Some(fallback_signal.to_string());
    }
    if request.strategy_reason.is_none() {
        request.strategy_reason = Some(format!(
            "{} {fallback_signal}",
            account.version.version_code
        ));
    }
    if request.config_hash.is_none() {
        request.config_hash = Some(account.version.config_hash.clone());
    }
    if request.tags.is_empty() {
        request.tags = vec![fallback_signal.to_string()];
    }
}

fn v3_version(now_ms: i64) -> StrategyVersion {
    let config_json = serde_json::json!({
        "base": "scalping_optimization_design",
        "version": V3_VERSION_CODE,
        "auto_open_score": AUTO_OPEN_SCORE,
        "paper_only": true
    });
    StrategyVersion {
        version_code: V3_VERSION_CODE.to_string(),
        name: "Scalping Base".to_string(),
        description: "Scalping Optimization Design v0.1.3 baseline".to_string(),
        status: StrategyVersionStatus::Active,
        config_hash: stable_config_hash(&config_json),
        config_json,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    }
}

fn v4_version(now_ms: i64) -> StrategyVersion {
    let config_json = serde_json::json!({
        "base": V3_VERSION_CODE,
        "version": V4_VERSION_CODE,
        "risk_guard": {
            "extreme_dump_gate": true,
            "same_symbol_cooldown_ms": 7200000,
            "no_add_to_loser": true,
            "account_kill_switch_drawdown": ACCOUNT_KILL_SWITCH_DRAWDOWN,
            "symbol_quarantine": ["LAB", "PUMP"],
            "risky_tag_downgrade": true
        },
        "paper_only": true
    });
    StrategyVersion {
        version_code: V4_VERSION_CODE.to_string(),
        name: "Risk Guard Edition".to_string(),
        description:
            "v0.1.4 + extreme dump filter + same-symbol cooldown + account kill switch + symbol quarantine"
                .to_string(),
        status: StrategyVersionStatus::Testing,
        config_hash: stable_config_hash(&config_json),
        config_json,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    }
}

fn stable_config_hash(value: &serde_json::Value) -> String {
    let json = serde_json::to_string(value).unwrap_or_default();
    let hash = json.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}

fn score_bucket(score: u8) -> String {
    match score {
        90..=100 => "score_90_100",
        85..=89 => "score_85_90",
        80..=84 => "score_80_85",
        _ => "score_below_80",
    }
    .to_string()
}

impl From<PaperTradeAction> for OrderAction {
    fn from(action: PaperTradeAction) -> Self {
        match action {
            PaperTradeAction::Open => Self::Open,
            PaperTradeAction::Close => Self::Close,
        }
    }
}
