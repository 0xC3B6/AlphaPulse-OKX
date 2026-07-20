use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    domain::{Direction, LevelKind, PatternSignal, PatternStatus, Score, SymbolSnapshot},
    market_context::{
        INTRADAY_DOWN_EXTREME, INTRADAY_UP_EXTREME, MULTIDAY_DOWN_EXTREME, MULTIDAY_UP_EXTREME,
    },
    paper::{
        automatic_trigger_prices, PaperAccountSnapshot, PaperOrderRequest, PaperPositionSnapshot,
        PaperSide, SCALPING_OPTIMIZATION_VERSION,
    },
    time_regime::{classify_time_regime, TradeTag, TradeTagKind},
};

const REVERSAL_CONFIRMATION_THRESHOLD: u8 = 76;
const REVERSAL_WATCH_CONFIRMATION_BOOST: u8 = 3;
const PATTERN_ENTRY_THRESHOLD: u8 = 75;
const PATTERN_STRUCTURE_THRESHOLD: u8 = 45;
const PATTERN_MIN_VOLUME_RATIO: f64 = 1.5;
const PATTERN_MAX_VWAP_DISTANCE_ATR: f64 = 2.0;
const PATTERN_MAX_STOP_ATR: f64 = 1.8;
const PATTERN_MIN_REWARD_RISK: f64 = 1.2;
const MOVER_MIN_VOLUME_RATIO: f64 = 2.0;
const VWAP_CHASE_PENALTY_ATR: f64 = 1.5;
const VWAP_CHASE_BLOCK_ATR: f64 = 2.5;
const MOVE_CHASE_PENALTY_ATR: f64 = 2.0;
const MOVE_CHASE_BLOCK_ATR: f64 = 3.0;
const FUNDING_CROWDING_THRESHOLD: f64 = 0.002;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AutoStrategyConfig {
    pub enabled: bool,
    pub default_leverage: f64,
    pub margin_fraction: f64,
    pub max_positions: usize,
    pub stop_loss_margin_pct: f64,
    pub take_profit_margin_pct: f64,
    pub allow_multiday_reversal_short: bool,
    pub allow_trend_long: bool,
    pub allow_trend_short: bool,
    pub allow_range_long: bool,
    pub allow_range_short: bool,
    pub allow_pattern_long: bool,
    pub allow_pattern_short: bool,
    pub allow_mover_long: bool,
    pub allow_mover_short: bool,
    pub trend_threshold: u8,
    pub range_threshold: u8,
    pub pattern_threshold: u8,
}

impl Default for AutoStrategyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_leverage: 20.0,
            margin_fraction: 0.03,
            max_positions: 5,
            stop_loss_margin_pct: -0.30,
            take_profit_margin_pct: 0.40,
            allow_multiday_reversal_short: true,
            allow_trend_long: true,
            allow_trend_short: true,
            allow_range_long: true,
            allow_range_short: true,
            allow_pattern_long: true,
            allow_pattern_short: true,
            allow_mover_long: true,
            allow_mover_short: true,
            trend_threshold: 80,
            range_threshold: 85,
            pattern_threshold: PATTERN_ENTRY_THRESHOLD,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AutoStrategyDecision {
    Open {
        order: PaperOrderRequest,
        reason: String,
        tags: Vec<TradeTag>,
    },
    Close {
        inst_id: String,
        reason: String,
        tags: Vec<TradeTag>,
        execution_price: Option<f64>,
        exit_kind: AutoExitKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoExitKind {
    StopLoss,
    TakeProfit,
}

pub fn evaluate_auto_strategy(
    symbol: &SymbolSnapshot,
    paper: &PaperAccountSnapshot,
    config: AutoStrategyConfig,
) -> Option<AutoStrategyDecision> {
    evaluate_auto_strategy_at(symbol, paper, config, Utc::now().timestamp_millis())
}

pub fn evaluate_auto_strategy_at(
    symbol: &SymbolSnapshot,
    paper: &PaperAccountSnapshot,
    config: AutoStrategyConfig,
    now_ms: i64,
) -> Option<AutoStrategyDecision> {
    if !config.enabled {
        return None;
    }
    let time_regime = classify_time_regime(now_ms);

    if let Some(decision) = evaluate_auto_exit(&symbol.inst_id, paper, config) {
        return Some(decision);
    }
    if paper
        .positions
        .iter()
        .any(|position| position.inst_id == symbol.inst_id)
    {
        return None;
    }
    if paper.positions.len() >= config.max_positions {
        return None;
    }

    let signal = open_signal(symbol, time_regime.score_penalty, now_ms, config)?;
    let margin = paper.equity * config.margin_fraction * signal.margin_multiplier;
    if margin <= 0.0 || paper.available_balance < margin {
        return None;
    }
    let mut tags = time_regime.tags;
    tags.extend(signal.tags);
    let reason = append_penalty_reason(signal.reason, time_regime.score_penalty, signal.penalty);
    let (stop_loss, take_profit) =
        automatic_trigger_prices(symbol.price, signal.side, config.default_leverage);
    let signal_tags = std::iter::once(signal.primary_signal.to_string())
        .chain(tags.iter().map(|tag| tag.label.clone()))
        .collect();

    Some(AutoStrategyDecision::Open {
        order: PaperOrderRequest::automatic(
            symbol.inst_id.clone(),
            signal.side,
            margin,
            config.default_leverage,
            stop_loss,
            take_profit,
            None,
            signal.primary_signal,
            reason.clone(),
            signal_tags,
        ),
        reason,
        tags,
    })
}

pub fn evaluate_auto_exit(
    inst_id: &str,
    paper: &PaperAccountSnapshot,
    config: AutoStrategyConfig,
) -> Option<AutoStrategyDecision> {
    if !config.enabled {
        return None;
    }
    let position = paper
        .positions
        .iter()
        .find(|position| position.inst_id == inst_id)?;

    if crossed_stop_loss(position) || position.pnl_pct <= config.stop_loss_margin_pct {
        return Some(AutoStrategyDecision::Close {
            inst_id: inst_id.to_string(),
            reason: strategy_reason(format_args!(
                "stop loss {:.2}%; trigger {:.2}%",
                position.pnl_pct * 100.0,
                config.stop_loss_margin_pct * 100.0
            )),
            tags: Vec::new(),
            execution_price: position
                .stop_loss
                .or_else(|| exit_trigger_price(position, config.stop_loss_margin_pct)),
            exit_kind: AutoExitKind::StopLoss,
        });
    }
    if crossed_take_profit(position) || position.pnl_pct >= config.take_profit_margin_pct {
        return Some(AutoStrategyDecision::Close {
            inst_id: inst_id.to_string(),
            reason: strategy_reason(format_args!(
                "take profit {:.2}%; trigger {:.2}%",
                position.pnl_pct * 100.0,
                config.take_profit_margin_pct * 100.0
            )),
            tags: Vec::new(),
            execution_price: position
                .take_profit
                .or_else(|| exit_trigger_price(position, config.take_profit_margin_pct)),
            exit_kind: AutoExitKind::TakeProfit,
        });
    }
    None
}

fn crossed_stop_loss(position: &PaperPositionSnapshot) -> bool {
    position
        .stop_loss
        .is_some_and(|stop_loss| match position.side {
            PaperSide::Long => position.mark_price <= stop_loss,
            PaperSide::Short => position.mark_price >= stop_loss,
        })
}

fn crossed_take_profit(position: &PaperPositionSnapshot) -> bool {
    position
        .take_profit
        .is_some_and(|take_profit| match position.side {
            PaperSide::Long => position.mark_price >= take_profit,
            PaperSide::Short => position.mark_price <= take_profit,
        })
}

fn exit_trigger_price(position: &PaperPositionSnapshot, target_margin_pct: f64) -> Option<f64> {
    if position.entry_price <= 0.0
        || !position.entry_price.is_finite()
        || position.leverage <= 0.0
        || !position.leverage.is_finite()
    {
        return None;
    }
    let raw_move = target_margin_pct / position.leverage;
    let price = match position.side {
        PaperSide::Long => position.entry_price * (1.0 + raw_move),
        PaperSide::Short => position.entry_price * (1.0 - raw_move),
    };
    (price > 0.0 && price.is_finite()).then_some(price)
}

struct OpenSignal {
    side: PaperSide,
    primary_signal: &'static str,
    reason: String,
    tags: Vec<TradeTag>,
    penalty: u8,
    margin_multiplier: f64,
}

fn open_signal(
    symbol: &SymbolSnapshot,
    time_penalty: u8,
    now_ms: i64,
    config: AutoStrategyConfig,
) -> Option<OpenSignal> {
    if config.allow_multiday_reversal_short {
        if let Some(signal) = multiday_extension_reversal_short(symbol, now_ms) {
            return Some(signal);
        }
    }
    if direction_allowed(
        symbol.trend_score.direction,
        config.allow_trend_long,
        config.allow_trend_short,
    ) {
        if let Some(signal) = score_signal(
            "trend",
            &symbol.trend_score,
            config.trend_threshold,
            time_penalty,
            symbol,
            now_ms,
        ) {
            return Some(signal);
        }
    }
    if direction_allowed(
        symbol.range_score.direction,
        config.allow_range_long,
        config.allow_range_short,
    ) {
        if let Some(signal) = score_signal(
            "range",
            &symbol.range_score,
            config.range_threshold,
            time_penalty,
            symbol,
            now_ms,
        ) {
            return Some(signal);
        }
    }
    if let Some(signal) = pattern_signal(symbol, time_penalty, now_ms, config) {
        return Some(signal);
    }
    if time_penalty > 0 {
        None
    } else {
        mover_signal(symbol, now_ms, config)
    }
}

fn multiday_extension_reversal_short(symbol: &SymbolSnapshot, now_ms: i64) -> Option<OpenSignal> {
    if !has_pool_tag(symbol, MULTIDAY_UP_EXTREME) {
        return None;
    }
    let in_reversal_watch_window = in_midday_reversal_window(now_ms);
    let confirmation_score = reversal_short_confirmation_score(symbol);
    let required_score = if in_reversal_watch_window {
        REVERSAL_CONFIRMATION_THRESHOLD.saturating_sub(REVERSAL_WATCH_CONFIRMATION_BOOST)
    } else {
        REVERSAL_CONFIRMATION_THRESHOLD
    };
    if confirmation_score < required_score {
        return None;
    }
    let mut tags = vec![probe_tag(
        now_ms,
        "multi-day extension reversal probe",
        "short after multi-day upside extension; use smaller probe margin first",
    )];
    if in_reversal_watch_window {
        tags.push(midday_reversal_tag(now_ms));
    }
    Some(OpenSignal {
        side: PaperSide::Short,
        primary_signal: "multiday_reversal_short",
        reason: strategy_reason(format_args!(
            "multiday extension reversal short {confirmation_score}"
        )),
        tags,
        penalty: 0,
        margin_multiplier: 0.6,
    })
}

fn reversal_short_confirmation_score(symbol: &SymbolSnapshot) -> u8 {
    let mut score = 0_u8;
    if symbol.trend_score.direction == Direction::Short {
        score = score.max(symbol.trend_score.value);
    }
    if symbol.range_score.direction == Direction::Short {
        score = score.max(symbol.range_score.value);
    }
    if symbol.change_15m_pct <= -0.025 && symbol.change_1h_pct <= -0.02 {
        score = score.max(72);
    }
    if symbol.change_15m_pct <= -0.04 && symbol.change_1h_pct <= -0.03 {
        score = score.max(82);
    }
    let pattern_score = symbol
        .pattern_signals
        .iter()
        .filter(|signal| {
            signal.direction == Direction::Short
                && matches!(
                    signal.status,
                    PatternStatus::Confirmed | PatternStatus::Retest | PatternStatus::Holding
                )
        })
        .map(|signal| signal.trade_score)
        .max()
        .unwrap_or(0);
    score.max(pattern_score)
}

fn in_midday_reversal_window(now_ms: i64) -> bool {
    let Some(now_utc) = DateTime::<Utc>::from_timestamp_millis(now_ms) else {
        return false;
    };
    let china_time = now_utc + Duration::hours(8);
    let minutes = china_time.hour() * 60 + china_time.minute();
    (14 * 60..15 * 60).contains(&minutes)
}

fn midday_reversal_tag(ts_ms: i64) -> TradeTag {
    TradeTag {
        kind: TradeTagKind::MiddayReversalWindow,
        label: "reversal watch timing".to_string(),
        score_impact: REVERSAL_WATCH_CONFIRMATION_BOOST as i32,
        reason: "14:00-15:00 China time is a light reversal-watch context; structure still must confirm direction".to_string(),
        ts_ms,
    }
}

fn score_signal(
    kind: &'static str,
    score: &Score,
    threshold: u8,
    time_penalty: u8,
    symbol: &SymbolSnapshot,
    now_ms: i64,
) -> Option<OpenSignal> {
    if score.direction == Direction::Neutral {
        return None;
    }
    let side = side_from_direction(score.direction)?;
    let context = entry_context(symbol, side, now_ms);
    if context.hard_block {
        return None;
    }
    let required_score = threshold
        .saturating_add(time_penalty)
        .saturating_add(context.score_penalty);
    if score.value < required_score {
        return None;
    }
    Some(OpenSignal {
        side,
        primary_signal: match (kind, side) {
            ("trend", PaperSide::Long) => "trend_long",
            ("trend", PaperSide::Short) => "trend_short",
            ("range", PaperSide::Long) => "range_long",
            ("range", PaperSide::Short) => "range_short",
            _ => "score_signal",
        },
        reason: strategy_reason(format_args!(
            "{kind} {} {}",
            direction_label(score.direction),
            score.value
        )),
        tags: context.tags,
        penalty: context.score_penalty,
        margin_multiplier: context.margin_multiplier * score_risk_multiplier(score.value),
    })
}

fn pattern_signal(
    symbol: &SymbolSnapshot,
    time_penalty: u8,
    now_ms: i64,
    config: AutoStrategyConfig,
) -> Option<OpenSignal> {
    let signal = symbol.pattern_signals.iter().find(|signal| {
        if signal.direction == Direction::Neutral
            || !matches!(
                signal.status,
                PatternStatus::Holding | PatternStatus::Retest
            )
            || !direction_allowed(
                signal.direction,
                config.allow_pattern_long,
                config.allow_pattern_short,
            )
        {
            return false;
        }
        let Some(side) = side_from_direction(signal.direction) else {
            return false;
        };
        let context = entry_context(symbol, side, now_ms);
        if context.hard_block || !pattern_quality_ok(symbol, signal) {
            return false;
        }
        let required_score = config
            .pattern_threshold
            .saturating_add(time_penalty)
            .saturating_add(context.score_penalty);
        signal.trade_score >= required_score && pattern_has_confluence(symbol, signal.direction)
    })?;
    let side = side_from_direction(signal.direction)?;
    let context = entry_context(symbol, side, now_ms);
    Some(OpenSignal {
        side,
        primary_signal: match side {
            PaperSide::Long => "pattern_long",
            PaperSide::Short => "pattern_short",
        },
        reason: strategy_reason(format_args!(
            "pattern {} {}",
            direction_label(signal.direction),
            signal.trade_score
        )),
        tags: context.tags,
        penalty: context.score_penalty,
        margin_multiplier: context.margin_multiplier * score_risk_multiplier(signal.trade_score),
    })
}

fn mover_signal(
    symbol: &SymbolSnapshot,
    now_ms: i64,
    config: AutoStrategyConfig,
) -> Option<OpenSignal> {
    if !symbol.pool_tags.iter().any(|tag| tag == "mover_24h") {
        return None;
    }
    let direction = if symbol.change_15m_pct > 0.0 && symbol.change_1h_pct > 0.0 {
        Direction::Long
    } else if symbol.change_15m_pct < 0.0 && symbol.change_1h_pct < 0.0 {
        Direction::Short
    } else {
        Direction::Neutral
    };
    if !direction_allowed(direction, config.allow_mover_long, config.allow_mover_short) {
        return None;
    }
    let side = side_from_direction(direction)?;
    let context = entry_context(symbol, side, now_ms);
    if context.hard_block || context.score_penalty > 0 || !mover_quality_ok(symbol, direction) {
        return None;
    }
    Some(OpenSignal {
        side,
        primary_signal: match side {
            PaperSide::Long => "mover_long",
            PaperSide::Short => "mover_short",
        },
        reason: strategy_reason(format_args!("24h mover {}", direction_label(direction))),
        tags: context.tags,
        penalty: context.score_penalty,
        margin_multiplier: context.margin_multiplier,
    })
}

fn direction_allowed(direction: Direction, allow_long: bool, allow_short: bool) -> bool {
    match direction {
        Direction::Long => allow_long,
        Direction::Short => allow_short,
        Direction::Neutral => false,
    }
}

struct EntryContext {
    tags: Vec<TradeTag>,
    score_penalty: u8,
    margin_multiplier: f64,
    hard_block: bool,
}

fn entry_context(symbol: &SymbolSnapshot, side: PaperSide, now_ms: i64) -> EntryContext {
    let intraday_up = has_pool_tag(symbol, INTRADAY_UP_EXTREME);
    let intraday_down = has_pool_tag(symbol, INTRADAY_DOWN_EXTREME);
    let multiday_up = has_pool_tag(symbol, MULTIDAY_UP_EXTREME);
    let multiday_down = has_pool_tag(symbol, MULTIDAY_DOWN_EXTREME);
    let mut tags = Vec::new();
    let mut score_penalty = 0_u8;
    let mut margin_multiplier = 1.0;
    let mut hard_block = false;

    match side {
        PaperSide::Long => {
            apply_location_risk(
                symbol,
                side,
                now_ms,
                &mut tags,
                &mut score_penalty,
                &mut hard_block,
            );
            if intraday_up {
                add_context_penalty(
                    &mut tags,
                    &mut score_penalty,
                    now_ms,
                    TradeTagKind::IntradayChaseLongRisk,
                    "intraday chase-long risk",
                    10,
                    "recent intraday upside extension makes chase longs lower quality",
                );
            }
            if multiday_up {
                add_context_penalty(
                    &mut tags,
                    &mut score_penalty,
                    now_ms,
                    TradeTagKind::MultiDayChaseLongRisk,
                    "multi-day chase-long risk",
                    15,
                    "multi-day upside extension requires stronger long confirmation",
                );
            }
            if intraday_down || multiday_down {
                margin_multiplier = 0.6;
                tags.push(probe_tag(
                    now_ms,
                    "downside extension reversal probe",
                    "long is counter-extension; use smaller probe margin first",
                ));
            }
        }
        PaperSide::Short => {
            apply_location_risk(
                symbol,
                side,
                now_ms,
                &mut tags,
                &mut score_penalty,
                &mut hard_block,
            );
            if intraday_down {
                add_context_penalty(
                    &mut tags,
                    &mut score_penalty,
                    now_ms,
                    TradeTagKind::IntradayChaseShortRisk,
                    "intraday chase-short risk",
                    10,
                    "recent intraday downside extension makes chase shorts lower quality",
                );
            }
            if multiday_down {
                add_context_penalty(
                    &mut tags,
                    &mut score_penalty,
                    now_ms,
                    TradeTagKind::MultiDayChaseShortRisk,
                    "multi-day chase-short risk",
                    15,
                    "multi-day downside extension requires stronger short confirmation",
                );
            }
            if intraday_up || multiday_up {
                margin_multiplier = 0.6;
                tags.push(probe_tag(
                    now_ms,
                    "upside extension reversal probe",
                    "short is counter-extension; use smaller probe margin first",
                ));
            }
        }
    }

    apply_funding_crowding_risk(symbol, side, now_ms, &mut tags, &mut score_penalty);
    if score_penalty > 0 {
        tags.push(TradeTag {
            kind: TradeTagKind::MarketPenaltyApplied,
            label: "market context penalty applied".to_string(),
            score_impact: -(score_penalty as i32),
            reason: format!("market context penalty {score_penalty}"),
            ts_ms: now_ms,
        });
    }

    EntryContext {
        tags,
        score_penalty,
        margin_multiplier,
        hard_block,
    }
}

fn apply_location_risk(
    symbol: &SymbolSnapshot,
    side: PaperSide,
    now_ms: i64,
    tags: &mut Vec<TradeTag>,
    score_penalty: &mut u8,
    hard_block: &mut bool,
) {
    match side {
        PaperSide::Long => {
            if symbol
                .scalping_metrics
                .vwap_distance_atr
                .is_some_and(|distance| distance >= VWAP_CHASE_BLOCK_ATR)
                || symbol
                    .scalping_metrics
                    .latest_move_atr
                    .is_some_and(|move_atr| move_atr >= MOVE_CHASE_BLOCK_ATR)
            {
                *hard_block = true;
                return;
            }
            if symbol
                .scalping_metrics
                .vwap_distance_atr
                .is_some_and(|distance| distance >= VWAP_CHASE_PENALTY_ATR)
            {
                add_context_penalty(
                    tags,
                    score_penalty,
                    now_ms,
                    TradeTagKind::VwapExtensionRisk,
                    "VWAP extension chase risk",
                    10,
                    "price is extended above VWAP by ATR-normalized distance",
                );
            }
            if symbol
                .scalping_metrics
                .latest_move_atr
                .is_some_and(|move_atr| move_atr >= MOVE_CHASE_PENALTY_ATR)
            {
                add_context_penalty(
                    tags,
                    score_penalty,
                    now_ms,
                    TradeTagKind::AtrImpulseRisk,
                    "ATR impulse chase risk",
                    8,
                    "latest candle move is extended versus ATR",
                );
            }
        }
        PaperSide::Short => {
            if symbol
                .scalping_metrics
                .vwap_distance_atr
                .is_some_and(|distance| distance <= -VWAP_CHASE_BLOCK_ATR)
                || symbol
                    .scalping_metrics
                    .latest_move_atr
                    .is_some_and(|move_atr| move_atr <= -MOVE_CHASE_BLOCK_ATR)
            {
                *hard_block = true;
                return;
            }
            if symbol
                .scalping_metrics
                .vwap_distance_atr
                .is_some_and(|distance| distance <= -VWAP_CHASE_PENALTY_ATR)
            {
                add_context_penalty(
                    tags,
                    score_penalty,
                    now_ms,
                    TradeTagKind::VwapExtensionRisk,
                    "VWAP extension chase risk",
                    10,
                    "price is extended below VWAP by ATR-normalized distance",
                );
            }
            if symbol
                .scalping_metrics
                .latest_move_atr
                .is_some_and(|move_atr| move_atr <= -MOVE_CHASE_PENALTY_ATR)
            {
                add_context_penalty(
                    tags,
                    score_penalty,
                    now_ms,
                    TradeTagKind::AtrImpulseRisk,
                    "ATR impulse chase risk",
                    8,
                    "latest candle move is extended versus ATR",
                );
            }
        }
    }
}

fn apply_funding_crowding_risk(
    symbol: &SymbolSnapshot,
    side: PaperSide,
    now_ms: i64,
    tags: &mut Vec<TradeTag>,
    score_penalty: &mut u8,
) {
    let Some(funding_rate) = symbol.funding_rate else {
        return;
    };
    let crowded = match side {
        PaperSide::Long => funding_rate >= FUNDING_CROWDING_THRESHOLD,
        PaperSide::Short => funding_rate <= -FUNDING_CROWDING_THRESHOLD,
    };
    if crowded {
        add_context_penalty(
            tags,
            score_penalty,
            now_ms,
            TradeTagKind::FundingCrowdingRisk,
            "funding crowding risk",
            8,
            "funding is extended in the same direction as the entry",
        );
    }
}

fn pattern_quality_ok(symbol: &SymbolSnapshot, signal: &PatternSignal) -> bool {
    signal.structure_score >= PATTERN_STRUCTURE_THRESHOLD
        && signal.trade_score >= PATTERN_ENTRY_THRESHOLD
        && signal.invalidation_level.is_some()
        && signal.confirm_ts_ms.is_some()
        && symbol.scalping_metrics.volume_ratio >= PATTERN_MIN_VOLUME_RATIO
        && pattern_vwap_location_ok(symbol, signal.direction)
        && pattern_stop_distance_ok(symbol, signal)
        && pattern_reward_risk_ok(symbol, signal)
}

fn pattern_vwap_location_ok(symbol: &SymbolSnapshot, direction: Direction) -> bool {
    let Some(distance) = symbol.scalping_metrics.vwap_distance_atr else {
        return true;
    };
    match direction {
        Direction::Long => distance <= PATTERN_MAX_VWAP_DISTANCE_ATR,
        Direction::Short => distance >= -PATTERN_MAX_VWAP_DISTANCE_ATR,
        Direction::Neutral => false,
    }
}

fn pattern_stop_distance_ok(symbol: &SymbolSnapshot, signal: &PatternSignal) -> bool {
    let Some(invalidation) = signal.invalidation_level else {
        return false;
    };
    if symbol.price <= 0.0 {
        return false;
    }
    let stop_distance_pct = (symbol.price - invalidation).abs() / symbol.price;
    let max_stop_pct = symbol
        .scalping_metrics
        .atr_15m_pct
        .map(|atr_pct| (atr_pct * PATTERN_MAX_STOP_ATR).max(0.018))
        .unwrap_or(0.018);
    stop_distance_pct <= max_stop_pct
}

fn pattern_reward_risk_ok(symbol: &SymbolSnapshot, signal: &PatternSignal) -> bool {
    pattern_reward_risk(symbol, signal)
        .is_some_and(|reward_risk| reward_risk >= PATTERN_MIN_REWARD_RISK)
}

fn pattern_reward_risk(symbol: &SymbolSnapshot, signal: &PatternSignal) -> Option<f64> {
    let neckline = signal.neckline?;
    let invalidation = signal.invalidation_level?;
    let height = (neckline - invalidation).abs();
    if height <= 0.0 || symbol.price <= 0.0 {
        return None;
    }
    let (reward, risk) = match signal.direction {
        Direction::Long => {
            let target = neckline + height;
            (target - symbol.price, symbol.price - invalidation)
        }
        Direction::Short => {
            let target = neckline - height;
            (symbol.price - target, invalidation - symbol.price)
        }
        Direction::Neutral => return None,
    };
    if reward <= 0.0 || risk <= 0.0 {
        return None;
    }
    Some(reward / risk)
}

fn pattern_has_confluence(symbol: &SymbolSnapshot, direction: Direction) -> bool {
    if symbol.trend_score.direction == direction && symbol.trend_score.value >= 55 {
        return true;
    }
    if symbol.range_score.direction == direction && symbol.range_score.value >= 55 {
        return true;
    }
    if symbol
        .fvgs
        .iter()
        .any(|zone| !zone.filled && zone.direction == direction && zone.distance_pct <= 0.012)
    {
        return true;
    }
    symbol.levels.iter().any(|level| match direction {
        Direction::Long => level.kind == LevelKind::Support && level.distance_pct <= 0.012,
        Direction::Short => level.kind == LevelKind::Resistance && level.distance_pct <= 0.012,
        Direction::Neutral => false,
    })
}

fn mover_quality_ok(symbol: &SymbolSnapshot, direction: Direction) -> bool {
    if symbol.scalping_metrics.volume_ratio < MOVER_MIN_VOLUME_RATIO {
        return false;
    }
    let Some(vwap_distance_atr) = symbol.scalping_metrics.vwap_distance_atr else {
        return false;
    };
    if vwap_distance_atr.abs() > VWAP_CHASE_PENALTY_ATR {
        return false;
    }
    match direction {
        Direction::Long => symbol
            .scalping_metrics
            .latest_move_atr
            .is_some_and(|move_atr| move_atr <= MOVE_CHASE_PENALTY_ATR),
        Direction::Short => symbol
            .scalping_metrics
            .latest_move_atr
            .is_some_and(|move_atr| move_atr >= -MOVE_CHASE_PENALTY_ATR),
        Direction::Neutral => false,
    }
}

fn score_risk_multiplier(score: u8) -> f64 {
    match score {
        95..=100 => 1.0,
        90..=94 => 0.6,
        _ => 0.3,
    }
}

fn has_pool_tag(symbol: &SymbolSnapshot, needle: &str) -> bool {
    symbol.pool_tags.iter().any(|tag| tag == needle)
}

#[allow(clippy::too_many_arguments)]
fn add_context_penalty(
    tags: &mut Vec<TradeTag>,
    score_penalty: &mut u8,
    ts_ms: i64,
    kind: TradeTagKind,
    label: &str,
    penalty: u8,
    reason: &str,
) {
    *score_penalty = score_penalty.saturating_add(penalty);
    tags.push(TradeTag {
        kind,
        label: label.to_string(),
        score_impact: -(penalty as i32),
        reason: reason.to_string(),
        ts_ms,
    });
}

fn probe_tag(ts_ms: i64, label: &str, reason: &str) -> TradeTag {
    TradeTag {
        kind: TradeTagKind::OverextensionReversalProbe,
        label: label.to_string(),
        score_impact: 0,
        reason: reason.to_string(),
        ts_ms,
    }
}

fn side_from_direction(direction: Direction) -> Option<PaperSide> {
    match direction {
        Direction::Long => Some(PaperSide::Long),
        Direction::Short => Some(PaperSide::Short),
        Direction::Neutral => None,
    }
}

fn direction_label(direction: Direction) -> &'static str {
    match direction {
        Direction::Long => "long",
        Direction::Short => "short",
        Direction::Neutral => "neutral",
    }
}

fn append_penalty_reason(reason: String, time_penalty: u8, context_penalty: u8) -> String {
    let mut reason = reason;
    if time_penalty > 0 {
        reason = format!("{reason}; time penalty {time_penalty}");
    }
    if context_penalty > 0 {
        reason = format!("{reason}; market context penalty {context_penalty}");
    }
    reason
}

fn strategy_reason(args: std::fmt::Arguments<'_>) -> String {
    format!("scalping {SCALPING_OPTIMIZATION_VERSION} {args}")
}
