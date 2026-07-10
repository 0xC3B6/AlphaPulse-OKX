use crate::domain::{
    Candle, Direction, PatternKind, PatternLevelZone, PatternPivot, PatternPivotRole,
    PatternSignal, PatternStatus, Timeframe,
};

const MAX_SCAN_BARS: usize = 80;

#[derive(Debug, Clone, Copy)]
struct PatternParams {
    pivot_window: usize,
    min_leg_bars: usize,
    max_width_bars: usize,
    atr: f64,
    tolerance: f64,
    confirm_buffer: f64,
    retest_zone: f64,
    hold_buffer: f64,
    min_height_pct: f64,
}

#[derive(Debug, Clone, Copy)]
struct PatternScores {
    structure: u8,
    confirmation: u8,
    hold: u8,
}

impl PatternScores {
    fn trade_score(self) -> u8 {
        self.structure
            .saturating_add(self.confirmation)
            .saturating_add(self.hold)
            .min(100)
    }
}

pub fn detect_patterns(
    candles: &[Candle],
    timeframe: Timeframe,
    current_price: f64,
) -> Vec<PatternSignal> {
    if candles.len() < 4 || current_price <= 0.0 {
        return Vec::new();
    }

    let start = candles.len().saturating_sub(MAX_SCAN_BARS);
    let candles = &candles[start..];
    let params = pattern_params(candles, timeframe, current_price);
    let mut signals = Vec::new();

    if let Some(signal) = detect_double_bottom(candles, timeframe, params) {
        signals.push(signal);
    }
    if let Some(signal) = detect_double_top(candles, timeframe, params) {
        signals.push(signal);
    }
    if let Some(signal) = detect_sweep_failure(candles, timeframe, params) {
        signals.push(signal);
    }

    signals.retain(|signal| signal.status != PatternStatus::Invalidated);
    signals.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| right.start_ts_ms.cmp(&left.start_ts_ms))
    });
    signals.truncate(4);
    signals
}

fn detect_double_bottom(
    candles: &[Candle],
    timeframe: Timeframe,
    params: PatternParams,
) -> Option<PatternSignal> {
    let mut best: Option<PatternSignal> = None;
    let touch_tolerance = (params.tolerance * 1.4).min(0.04);

    for left_index in 0..candles.len().saturating_sub(4) {
        if !is_local_low(candles, left_index, params.pivot_window) {
            continue;
        }
        for neck_index in (left_index + 1)..candles.len().saturating_sub(2) {
            let neckline = candles[neck_index].high;
            for right_index in (neck_index + 1)..candles.len().saturating_sub(1) {
                let leg_bars = right_index.saturating_sub(left_index);
                if leg_bars < params.min_leg_bars || leg_bars > params.max_width_bars {
                    continue;
                }
                if !is_local_low(candles, right_index, params.pivot_window) {
                    continue;
                }
                let left_low = candles[left_index].low;
                let right_low = candles[right_index].low;
                let higher_low = left_low.max(right_low);
                if pct_distance(left_low, right_low) > touch_tolerance {
                    continue;
                }
                if neckline <= higher_low * (1.0 + params.tolerance * 1.8) {
                    continue;
                }
                if !is_highest_between(candles, neck_index, left_index + 1, right_index) {
                    continue;
                }
                if !pattern_height_ok(left_low, right_low, neckline, params) {
                    continue;
                }

                let after_right = &candles[(right_index + 1)..];
                let confirm_offset = after_right
                    .iter()
                    .position(|candle| candle.close > neckline + params.confirm_buffer);
                let confirm_index = confirm_offset.map(|offset| right_index + 1 + offset);
                let status = double_bottom_status(
                    candles,
                    right_index,
                    confirm_index,
                    neckline,
                    right_low,
                    params,
                );
                let scores = score_double_pattern(
                    candles,
                    left_index,
                    neck_index,
                    right_index,
                    confirm_index,
                    status,
                    left_low,
                    right_low,
                    neckline,
                    params,
                    Direction::Long,
                );
                let score = scores.trade_score();
                let reasons = double_bottom_reasons(status, confirm_index.is_some());
                let signal = PatternSignal {
                    kind: PatternKind::DoubleBottom,
                    direction: Direction::Long,
                    timeframe,
                    status,
                    score,
                    structure_score: scores.structure,
                    confirmation_score: scores.confirmation,
                    hold_score: scores.hold,
                    trade_score: score,
                    neckline: Some(neckline),
                    invalidation_level: Some(right_low),
                    start_ts_ms: candles[left_index].ts_ms,
                    confirm_ts_ms: confirm_index.map(|index| candles[index].ts_ms),
                    pivots: vec![
                        PatternPivot {
                            role: PatternPivotRole::LeftLow,
                            ts_ms: candles[left_index].ts_ms,
                            price: left_low,
                        },
                        PatternPivot {
                            role: PatternPivotRole::Neckline,
                            ts_ms: candles[neck_index].ts_ms,
                            price: neckline,
                        },
                        PatternPivot {
                            role: PatternPivotRole::RightLow,
                            ts_ms: candles[right_index].ts_ms,
                            price: right_low,
                        },
                    ],
                    level_zone: Some(level_zone(neckline, params.tolerance)),
                    reasons,
                    warnings: Vec::new(),
                };
                best = choose_better(best, signal);
            }
        }
    }

    best
}

fn detect_double_top(
    candles: &[Candle],
    timeframe: Timeframe,
    params: PatternParams,
) -> Option<PatternSignal> {
    let mut best: Option<PatternSignal> = None;
    let touch_tolerance = (params.tolerance * 1.4).min(0.04);

    for left_index in 0..candles.len().saturating_sub(4) {
        if !is_local_high(candles, left_index, params.pivot_window) {
            continue;
        }
        for neck_index in (left_index + 1)..candles.len().saturating_sub(2) {
            let neckline = candles[neck_index].low;
            for right_index in (neck_index + 1)..candles.len().saturating_sub(1) {
                let leg_bars = right_index.saturating_sub(left_index);
                if leg_bars < params.min_leg_bars || leg_bars > params.max_width_bars {
                    continue;
                }
                if !is_local_high(candles, right_index, params.pivot_window) {
                    continue;
                }
                let left_high = candles[left_index].high;
                let right_high = candles[right_index].high;
                let lower_high = left_high.min(right_high);
                if pct_distance(left_high, right_high) > touch_tolerance {
                    continue;
                }
                if neckline >= lower_high * (1.0 - params.tolerance * 1.8) {
                    continue;
                }
                if !is_lowest_between(candles, neck_index, left_index + 1, right_index) {
                    continue;
                }
                if !pattern_height_ok(left_high, right_high, neckline, params) {
                    continue;
                }

                let after_right = &candles[(right_index + 1)..];
                let confirm_offset = after_right
                    .iter()
                    .position(|candle| candle.close < neckline - params.confirm_buffer);
                let confirm_index = confirm_offset.map(|offset| right_index + 1 + offset);
                let status = double_top_status(
                    candles,
                    right_index,
                    confirm_index,
                    neckline,
                    right_high,
                    params,
                );
                let scores = score_double_pattern(
                    candles,
                    left_index,
                    neck_index,
                    right_index,
                    confirm_index,
                    status,
                    left_high,
                    right_high,
                    neckline,
                    params,
                    Direction::Short,
                );
                let score = scores.trade_score();
                let reasons = double_top_reasons(status, confirm_index.is_some());
                let signal = PatternSignal {
                    kind: PatternKind::DoubleTop,
                    direction: Direction::Short,
                    timeframe,
                    status,
                    score,
                    structure_score: scores.structure,
                    confirmation_score: scores.confirmation,
                    hold_score: scores.hold,
                    trade_score: score,
                    neckline: Some(neckline),
                    invalidation_level: Some(right_high),
                    start_ts_ms: candles[left_index].ts_ms,
                    confirm_ts_ms: confirm_index.map(|index| candles[index].ts_ms),
                    pivots: vec![
                        PatternPivot {
                            role: PatternPivotRole::LeftHigh,
                            ts_ms: candles[left_index].ts_ms,
                            price: left_high,
                        },
                        PatternPivot {
                            role: PatternPivotRole::Neckline,
                            ts_ms: candles[neck_index].ts_ms,
                            price: neckline,
                        },
                        PatternPivot {
                            role: PatternPivotRole::RightHigh,
                            ts_ms: candles[right_index].ts_ms,
                            price: right_high,
                        },
                    ],
                    level_zone: Some(level_zone(neckline, params.tolerance)),
                    reasons,
                    warnings: Vec::new(),
                };
                best = choose_better(best, signal);
            }
        }
    }

    best
}

fn detect_sweep_failure(
    candles: &[Candle],
    timeframe: Timeframe,
    params: PatternParams,
) -> Option<PatternSignal> {
    let high_sweep = detect_high_sweep_failure(candles, timeframe, params);
    let low_sweep = detect_low_sweep_failure(candles, timeframe, params);
    match (high_sweep, low_sweep) {
        (Some(high), Some(low)) => Some(if high.score >= low.score { high } else { low }),
        (Some(high), None) => Some(high),
        (None, Some(low)) => Some(low),
        (None, None) => None,
    }
}

fn detect_high_sweep_failure(
    candles: &[Candle],
    timeframe: Timeframe,
    params: PatternParams,
) -> Option<PatternSignal> {
    let mut best = None;
    for reference_index in 0..candles.len().saturating_sub(1) {
        let reference_high = candles[reference_index].high;
        for sweep_index in (reference_index + 1)..candles.len() {
            let sweep = &candles[sweep_index];
            let sweep_break = sweep.high - reference_high;
            if sweep_break < sweep_break_buffer(reference_high, params) {
                continue;
            }
            if sweep.close >= reference_high {
                continue;
            }
            if upper_wick_ratio(sweep) < 0.40 {
                continue;
            }
            if candle_close_position(sweep) > 0.5 {
                continue;
            }
            if !volume_expanded(candles, sweep_index, 1.2) {
                continue;
            }
            let after_sweep = &candles[(sweep_index + 1)..];
            let invalidated = after_sweep
                .iter()
                .any(|candle| candle.close > sweep.high + params.hold_buffer);
            let holding = after_sweep.first().is_some_and(|next| {
                next.close < reference_high - params.hold_buffer
                    && next.high <= sweep.high + params.hold_buffer
            });
            let status = if invalidated {
                PatternStatus::Invalidated
            } else if holding {
                PatternStatus::Holding
            } else {
                PatternStatus::Confirmed
            };
            let scores = score_sweep_pattern(candles, reference_index, sweep_index, status, params);
            let score = scores.trade_score();
            let signal = PatternSignal {
                kind: PatternKind::SweepFailure,
                direction: Direction::Short,
                timeframe,
                status,
                score,
                structure_score: scores.structure,
                confirmation_score: scores.confirmation,
                hold_score: scores.hold,
                trade_score: score,
                neckline: Some(reference_high),
                invalidation_level: Some(sweep.high),
                start_ts_ms: candles[reference_index].ts_ms,
                confirm_ts_ms: Some(sweep.ts_ms),
                pivots: vec![
                    PatternPivot {
                        role: PatternPivotRole::SweepReference,
                        ts_ms: candles[reference_index].ts_ms,
                        price: reference_high,
                    },
                    PatternPivot {
                        role: PatternPivotRole::Sweep,
                        ts_ms: sweep.ts_ms,
                        price: sweep.high,
                    },
                ],
                level_zone: Some(level_zone(reference_high, params.tolerance)),
                reasons: vec![
                    "swept recent high and closed back below".to_string(),
                    if status == PatternStatus::Holding {
                        "failed reclaim is holding".to_string()
                    } else {
                        "failed reclaim confirmed".to_string()
                    },
                ],
                warnings: Vec::new(),
            };
            best = choose_better(best, signal);
        }
    }
    best
}

fn detect_low_sweep_failure(
    candles: &[Candle],
    timeframe: Timeframe,
    params: PatternParams,
) -> Option<PatternSignal> {
    let mut best = None;
    for reference_index in 0..candles.len().saturating_sub(1) {
        let reference_low = candles[reference_index].low;
        for sweep_index in (reference_index + 1)..candles.len() {
            let sweep = &candles[sweep_index];
            let sweep_break = reference_low - sweep.low;
            if sweep_break < sweep_break_buffer(reference_low, params) {
                continue;
            }
            if sweep.close <= reference_low {
                continue;
            }
            if lower_wick_ratio(sweep) < 0.40 {
                continue;
            }
            if candle_close_position(sweep) < 0.5 {
                continue;
            }
            if !volume_expanded(candles, sweep_index, 1.2) {
                continue;
            }
            let after_sweep = &candles[(sweep_index + 1)..];
            let invalidated = after_sweep
                .iter()
                .any(|candle| candle.close < sweep.low - params.hold_buffer);
            let holding = after_sweep.first().is_some_and(|next| {
                next.close > reference_low + params.hold_buffer
                    && next.low >= sweep.low - params.hold_buffer
            });
            let status = if invalidated {
                PatternStatus::Invalidated
            } else if holding {
                PatternStatus::Holding
            } else {
                PatternStatus::Confirmed
            };
            let scores = score_sweep_pattern(candles, reference_index, sweep_index, status, params);
            let score = scores.trade_score();
            let signal = PatternSignal {
                kind: PatternKind::SweepFailure,
                direction: Direction::Long,
                timeframe,
                status,
                score,
                structure_score: scores.structure,
                confirmation_score: scores.confirmation,
                hold_score: scores.hold,
                trade_score: score,
                neckline: Some(reference_low),
                invalidation_level: Some(sweep.low),
                start_ts_ms: candles[reference_index].ts_ms,
                confirm_ts_ms: Some(sweep.ts_ms),
                pivots: vec![
                    PatternPivot {
                        role: PatternPivotRole::SweepReference,
                        ts_ms: candles[reference_index].ts_ms,
                        price: reference_low,
                    },
                    PatternPivot {
                        role: PatternPivotRole::Sweep,
                        ts_ms: sweep.ts_ms,
                        price: sweep.low,
                    },
                ],
                level_zone: Some(level_zone(reference_low, params.tolerance)),
                reasons: vec![
                    "swept recent low and closed back above".to_string(),
                    if status == PatternStatus::Holding {
                        "failed breakdown is holding".to_string()
                    } else {
                        "failed breakdown confirmed".to_string()
                    },
                ],
                warnings: Vec::new(),
            };
            best = choose_better(best, signal);
        }
    }
    best
}

fn double_bottom_status(
    candles: &[Candle],
    right_index: usize,
    confirm_index: Option<usize>,
    neckline: f64,
    right_low: f64,
    params: PatternParams,
) -> PatternStatus {
    let Some(confirm_index) = confirm_index else {
        return PatternStatus::Forming;
    };
    let after_confirm = &candles[(confirm_index + 1)..];
    if after_confirm
        .iter()
        .any(|candle| candle.close < right_low - params.hold_buffer)
    {
        return PatternStatus::Invalidated;
    }
    let retest_index = after_confirm.iter().position(|candle| {
        candle.low <= neckline + params.retest_zone && candle.high >= neckline - params.retest_zone
    });
    if let Some(offset) = retest_index {
        let absolute_index = confirm_index + 1 + offset;
        let retest_close_held = candles[absolute_index].close >= neckline - params.hold_buffer;
        let next_close_held = candles
            .get(absolute_index + 1)
            .is_some_and(|next| next.close >= neckline);
        return if retest_close_held && next_close_held {
            PatternStatus::Holding
        } else {
            PatternStatus::Retest
        };
    }
    if candles
        .iter()
        .skip(right_index + 1)
        .any(|candle| candle.close >= neckline)
    {
        PatternStatus::Confirmed
    } else {
        PatternStatus::Forming
    }
}

fn double_top_status(
    candles: &[Candle],
    right_index: usize,
    confirm_index: Option<usize>,
    neckline: f64,
    right_high: f64,
    params: PatternParams,
) -> PatternStatus {
    let Some(confirm_index) = confirm_index else {
        return PatternStatus::Forming;
    };
    let after_confirm = &candles[(confirm_index + 1)..];
    if after_confirm
        .iter()
        .any(|candle| candle.close > right_high + params.hold_buffer)
    {
        return PatternStatus::Invalidated;
    }
    let retest_index = after_confirm.iter().position(|candle| {
        candle.high >= neckline - params.retest_zone && candle.low <= neckline + params.retest_zone
    });
    if let Some(offset) = retest_index {
        let absolute_index = confirm_index + 1 + offset;
        let retest_close_held = candles[absolute_index].close <= neckline + params.hold_buffer;
        let next_close_held = candles
            .get(absolute_index + 1)
            .is_some_and(|next| next.close <= neckline);
        return if retest_close_held && next_close_held {
            PatternStatus::Holding
        } else {
            PatternStatus::Retest
        };
    }
    if candles
        .iter()
        .skip(right_index + 1)
        .any(|candle| candle.close <= neckline)
    {
        PatternStatus::Confirmed
    } else {
        PatternStatus::Forming
    }
}

fn double_bottom_reasons(status: PatternStatus, confirmed: bool) -> Vec<String> {
    let mut reasons = vec!["double bottom geometry".to_string()];
    if confirmed {
        reasons.push("closed above neckline".to_string());
    }
    match status {
        PatternStatus::Holding => reasons.push("neckline retest holding".to_string()),
        PatternStatus::Retest => reasons.push("neckline retest in progress".to_string()),
        PatternStatus::Confirmed => reasons.push("breakout confirmed".to_string()),
        PatternStatus::Forming => reasons.push("awaiting neckline break".to_string()),
        PatternStatus::Invalidated => reasons.push("pattern invalidated".to_string()),
    }
    reasons
}

fn double_top_reasons(status: PatternStatus, confirmed: bool) -> Vec<String> {
    let mut reasons = vec!["double top geometry".to_string()];
    if confirmed {
        reasons.push("closed below neckline".to_string());
    }
    match status {
        PatternStatus::Holding => reasons.push("neckline retest rejecting".to_string()),
        PatternStatus::Retest => reasons.push("neckline retest in progress".to_string()),
        PatternStatus::Confirmed => reasons.push("breakdown confirmed".to_string()),
        PatternStatus::Forming => reasons.push("awaiting neckline break".to_string()),
        PatternStatus::Invalidated => reasons.push("pattern invalidated".to_string()),
    }
    reasons
}

fn choose_better(
    current: Option<PatternSignal>,
    candidate: PatternSignal,
) -> Option<PatternSignal> {
    match current {
        Some(current)
            if current.score > candidate.score
                || (current.score == candidate.score
                    && current.start_ts_ms >= candidate.start_ts_ms) =>
        {
            Some(current)
        }
        _ => Some(candidate),
    }
}

#[allow(clippy::too_many_arguments)]
fn score_double_pattern(
    candles: &[Candle],
    left_index: usize,
    neck_index: usize,
    right_index: usize,
    confirm_index: Option<usize>,
    status: PatternStatus,
    left: f64,
    right: f64,
    neckline: f64,
    params: PatternParams,
    direction: Direction,
) -> PatternScores {
    PatternScores {
        structure: double_structure_score(
            left_index,
            neck_index,
            right_index,
            left,
            right,
            neckline,
            params,
        ),
        confirmation: confirmation_score(candles, confirm_index, neckline, params, direction),
        hold: hold_score(
            candles,
            confirm_index,
            status,
            neckline,
            right,
            params,
            direction,
        ),
    }
}

fn double_structure_score(
    left_index: usize,
    neck_index: usize,
    right_index: usize,
    left: f64,
    right: f64,
    neckline: f64,
    params: PatternParams,
) -> u8 {
    let pivot_quality = 12.0;
    let symmetry = (1.0 - (pct_distance(left, right) / 0.04).min(1.0)) * 10.0;
    let avg_pivot = (left + right) / 2.0;
    let height = (neckline - avg_pivot).abs();
    let min_height = (params.atr * 1.2).max(avg_pivot.abs() * params.min_height_pct);
    let neckline_height = (height / min_height.max(0.00000001)).min(1.0) * 12.0;
    let width = right_index.saturating_sub(left_index);
    let width_reasonable = if width >= params.min_leg_bars && width <= params.max_width_bars {
        8.0
    } else {
        0.0
    };
    let middle_balance = {
        let left_leg = neck_index.saturating_sub(left_index).max(1);
        let right_leg = right_index.saturating_sub(neck_index).max(1);
        let ratio = left_leg.min(right_leg) as f64 / left_leg.max(right_leg) as f64;
        ratio * 8.0
    };
    let clean_structure = 10.0;
    (pivot_quality
        + symmetry
        + neckline_height
        + width_reasonable
        + middle_balance
        + clean_structure)
        .round()
        .clamp(0.0, 60.0) as u8
}

fn confirmation_score(
    candles: &[Candle],
    confirm_index: Option<usize>,
    neckline: f64,
    params: PatternParams,
    direction: Direction,
) -> u8 {
    let Some(confirm_index) = confirm_index else {
        return 0;
    };
    let candle = &candles[confirm_index];
    let mut score = 6_u8;
    let break_distance = match direction {
        Direction::Long => candle.close - neckline,
        Direction::Short => neckline - candle.close,
        Direction::Neutral => 0.0,
    };
    if break_distance >= params.atr * 0.15 {
        score += 4;
    }
    if directional_body_quality(candle, direction) {
        score += 4;
    }
    if volume_expanded(candles, confirm_index, 1.2) {
        score += 4;
    }
    let no_immediate_failure = candles
        .get(confirm_index + 1)
        .is_none_or(|next| match direction {
            Direction::Long => next.close >= neckline - params.hold_buffer,
            Direction::Short => next.close <= neckline + params.hold_buffer,
            Direction::Neutral => true,
        });
    if no_immediate_failure {
        score += 2;
    }
    score.min(20)
}

fn hold_score(
    candles: &[Candle],
    confirm_index: Option<usize>,
    status: PatternStatus,
    neckline: f64,
    invalidation: f64,
    params: PatternParams,
    direction: Direction,
) -> u8 {
    if !matches!(status, PatternStatus::Retest | PatternStatus::Holding) {
        return 0;
    }
    let Some(confirm_index) = confirm_index else {
        return 0;
    };
    let Some(retest_index) = find_retest_index(candles, confirm_index, neckline, params) else {
        return 0;
    };
    let retest = &candles[retest_index];
    let mut score = 5_u8;
    let retest_close_held = match direction {
        Direction::Long => retest.close >= neckline - params.hold_buffer,
        Direction::Short => retest.close <= neckline + params.hold_buffer,
        Direction::Neutral => false,
    };
    if retest_close_held {
        score += 5;
    }
    if volume_contracted(candles, retest_index, 0.95) {
        score += 4;
    }
    let next_held = candles
        .get(retest_index + 1)
        .is_some_and(|next| match direction {
            Direction::Long => next.close >= neckline,
            Direction::Short => next.close <= neckline,
            Direction::Neutral => false,
        });
    if next_held {
        score += 4;
    }
    let stop_distance = (neckline - invalidation).abs();
    if stop_distance <= params.atr * 1.8 {
        score += 2;
    }
    score.min(20)
}

fn score_sweep_pattern(
    candles: &[Candle],
    reference_index: usize,
    sweep_index: usize,
    status: PatternStatus,
    params: PatternParams,
) -> PatternScores {
    let sweep = &candles[sweep_index];
    let wick_quality = upper_wick_ratio(sweep).max(lower_wick_ratio(sweep));
    let break_distance = (sweep.high - candles[reference_index].high)
        .max(candles[reference_index].low - sweep.low)
        .max(0.0);
    let structure = (16.0
        + (break_distance / sweep_break_buffer(sweep.close, params).max(0.00000001)).min(1.0)
            * 12.0
        + wick_quality.min(0.7) / 0.7 * 16.0
        + 10.0)
        .round()
        .clamp(0.0, 60.0) as u8;
    let confirmation = if volume_expanded(candles, sweep_index, 1.2) {
        18
    } else {
        14
    };
    let hold = if status == PatternStatus::Holding {
        18
    } else {
        0
    };
    PatternScores {
        structure,
        confirmation,
        hold,
    }
}

fn level_zone(level: f64, tolerance: f64) -> PatternLevelZone {
    let width = tolerance * 0.55;
    PatternLevelZone {
        lower: level * (1.0 - width),
        upper: level * (1.0 + width),
    }
}

fn find_retest_index(
    candles: &[Candle],
    confirm_index: usize,
    neckline: f64,
    params: PatternParams,
) -> Option<usize> {
    candles[(confirm_index + 1)..]
        .iter()
        .position(|candle| {
            candle.low <= neckline + params.retest_zone
                && candle.high >= neckline - params.retest_zone
        })
        .map(|offset| confirm_index + 1 + offset)
}

fn pattern_height_ok(left: f64, right: f64, neckline: f64, params: PatternParams) -> bool {
    let avg_pivot = ((left + right) / 2.0).abs();
    let height = (neckline - ((left + right) / 2.0)).abs();
    let min_height = (params.atr * 1.2).max(avg_pivot * params.min_height_pct);
    let max_height = (params.atr * 5.0).max(avg_pivot * 0.004);
    height >= min_height && height <= max_height
}

fn is_local_low(candles: &[Candle], index: usize, window: usize) -> bool {
    if candles.is_empty() || index < window || index + window >= candles.len() {
        return false;
    }
    let low = candles[index].low;
    let left = index - window;
    let right = index + window;
    (left..=right).all(|candidate| candidate == index || candles[candidate].low >= low)
}

fn is_local_high(candles: &[Candle], index: usize, window: usize) -> bool {
    if candles.is_empty() || index < window || index + window >= candles.len() {
        return false;
    }
    let high = candles[index].high;
    let left = index - window;
    let right = index + window;
    (left..=right).all(|candidate| candidate == index || candles[candidate].high <= high)
}

fn is_highest_between(candles: &[Candle], index: usize, start: usize, end: usize) -> bool {
    let high = candles[index].high;
    candles[start..=end]
        .iter()
        .all(|candle| candle.high <= high)
}

fn is_lowest_between(candles: &[Candle], index: usize, start: usize, end: usize) -> bool {
    let low = candles[index].low;
    candles[start..=end].iter().all(|candle| candle.low >= low)
}

fn pattern_params(candles: &[Candle], timeframe: Timeframe, current_price: f64) -> PatternParams {
    let atr = average_true_range(candles, 14).unwrap_or_else(|| current_price * 0.008);
    let tolerance = pattern_tolerance(candles, current_price);
    let (pivot_window, min_leg_bars, max_width_bars, min_height_pct) = match timeframe {
        Timeframe::M15 => (2, 8, 60, 0.008),
        Timeframe::H1 => (2, 5, 50, 0.008),
        _ => (2, 6, 50, 0.008),
    };
    let confirm_buffer = (atr * 0.15).max(current_price * 0.0015);
    PatternParams {
        pivot_window,
        min_leg_bars,
        max_width_bars,
        atr,
        tolerance,
        confirm_buffer,
        retest_zone: (atr * 0.35).max(current_price * tolerance),
        hold_buffer: (atr * 0.10).max(current_price * 0.0008),
        min_height_pct,
    }
}

fn pattern_tolerance(candles: &[Candle], current_price: f64) -> f64 {
    let recent = candles.iter().rev().take(32);
    let high = recent
        .clone()
        .map(|candle| candle.high)
        .fold(f64::NEG_INFINITY, f64::max);
    let low = candles
        .iter()
        .rev()
        .take(32)
        .map(|candle| candle.low)
        .fold(f64::INFINITY, f64::min);
    if !high.is_finite() || !low.is_finite() || current_price <= 0.0 {
        return 0.008;
    }
    (((high - low).max(0.0) / current_price) * 0.18).clamp(0.003, 0.025)
}

fn average_true_range(candles: &[Candle], period: usize) -> Option<f64> {
    if candles.len() < 2 {
        return None;
    }
    let mut ranges = Vec::new();
    for index in 1..candles.len() {
        let candle = &candles[index];
        let previous_close = candles[index - 1].close;
        let true_range = (candle.high - candle.low)
            .max((candle.high - previous_close).abs())
            .max((candle.low - previous_close).abs());
        if true_range.is_finite() && true_range > 0.0 {
            ranges.push(true_range);
        }
    }
    if ranges.is_empty() {
        return None;
    }
    let start = ranges.len().saturating_sub(period);
    let recent = &ranges[start..];
    Some(recent.iter().sum::<f64>() / recent.len() as f64)
}

fn pct_distance(left: f64, right: f64) -> f64 {
    let denominator = ((left.abs() + right.abs()) / 2.0).max(0.00000001);
    (left - right).abs() / denominator
}

fn directional_body_quality(candle: &Candle, direction: Direction) -> bool {
    let range = (candle.high - candle.low).max(0.00000001);
    let body = (candle.close - candle.open).abs();
    let body_ratio = body / range;
    match direction {
        Direction::Long => candle.close > candle.open && body_ratio >= 0.35,
        Direction::Short => candle.close < candle.open && body_ratio >= 0.35,
        Direction::Neutral => false,
    }
}

fn volume_expanded(candles: &[Candle], index: usize, multiplier: f64) -> bool {
    let Some(avg) = average_volume_before(candles, index, 20) else {
        return false;
    };
    candles[index].volume >= avg * multiplier
}

fn volume_contracted(candles: &[Candle], index: usize, multiplier: f64) -> bool {
    let Some(avg) = average_volume_before(candles, index, 20) else {
        return false;
    };
    candles[index].volume <= avg * multiplier
}

fn average_volume_before(candles: &[Candle], index: usize, period: usize) -> Option<f64> {
    if index == 0 {
        return None;
    }
    let start = index.saturating_sub(period);
    let window = &candles[start..index];
    if window.is_empty() {
        return None;
    }
    Some(window.iter().map(|candle| candle.volume).sum::<f64>() / window.len() as f64)
}

fn sweep_break_buffer(reference: f64, params: PatternParams) -> f64 {
    (params.atr * 0.15).max(reference.abs() * 0.001)
}

fn upper_wick_ratio(candle: &Candle) -> f64 {
    let range = (candle.high - candle.low).max(0.00000001);
    (candle.high - candle.open.max(candle.close)).max(0.0) / range
}

fn lower_wick_ratio(candle: &Candle) -> f64 {
    let range = (candle.high - candle.low).max(0.00000001);
    (candle.open.min(candle.close) - candle.low).max(0.0) / range
}

fn candle_close_position(candle: &Candle) -> f64 {
    let range = (candle.high - candle.low).max(0.00000001);
    ((candle.close - candle.low) / range).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Candle, Direction, PatternKind, PatternStatus, Timeframe};

    fn candle(index: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        candle_with_volume(index, open, high, low, close, 100.0 + index as f64)
    }

    fn candle_with_volume(
        index: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Candle {
        Candle {
            ts_ms: index * 60_000,
            open,
            high,
            low,
            close,
            volume,
        }
    }

    #[test]
    fn detects_double_bottom_retest_holding() {
        let candles = vec![
            candle(0, 100.0, 101.0, 99.0, 100.0),
            candle(1, 100.0, 101.0, 98.5, 99.5),
            candle(2, 99.5, 100.2, 98.2, 99.0),
            candle(3, 99.0, 100.0, 96.0, 97.0),
            candle(4, 97.0, 99.0, 96.8, 98.8),
            candle(5, 98.8, 101.0, 98.5, 100.0),
            candle(6, 100.0, 103.0, 99.5, 102.0),
            candle(7, 102.0, 104.0, 101.5, 103.5),
            candle(8, 103.5, 103.8, 101.0, 102.0),
            candle(9, 102.0, 102.5, 99.2, 100.0),
            candle(10, 100.0, 101.0, 97.2, 98.5),
            candle(11, 98.5, 100.0, 96.3, 97.8),
            candle(12, 97.8, 101.0, 97.5, 100.5),
            candle(13, 100.5, 105.2, 100.0, 104.8),
            candle(14, 104.8, 105.0, 103.8, 104.0),
            candle(15, 104.0, 105.5, 103.9, 104.8),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 104.8);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::DoubleBottom)
            .expect("double bottom should be detected");
        assert_eq!(signal.direction, Direction::Long);
        assert_eq!(signal.status, PatternStatus::Holding);
        assert_eq!(signal.neckline, Some(104.0));
        assert!(signal.score >= 60);
        assert!(signal
            .reasons
            .iter()
            .any(|reason| reason.contains("neckline retest holding")));
    }

    #[test]
    fn detects_double_top_retest_holding() {
        let candles = vec![
            candle(0, 100.0, 101.0, 99.0, 100.0),
            candle(1, 100.0, 102.0, 99.0, 101.0),
            candle(2, 101.0, 104.0, 100.0, 103.0),
            candle(3, 103.0, 106.0, 102.0, 105.0),
            candle(4, 105.0, 105.2, 101.0, 102.0),
            candle(5, 102.0, 103.0, 99.0, 100.0),
            candle(6, 100.0, 101.0, 97.0, 98.0),
            candle(7, 98.0, 100.0, 96.0, 97.0),
            candle(8, 97.0, 101.0, 96.8, 100.0),
            candle(9, 100.0, 103.0, 97.0, 102.0),
            candle(10, 102.0, 105.0, 99.0, 104.0),
            candle(11, 104.0, 105.8, 101.0, 105.0),
            candle(12, 105.0, 105.1, 100.0, 101.0),
            candle(13, 101.0, 101.2, 94.8, 95.0),
            candle(14, 95.0, 96.2, 94.5, 95.8),
            candle(15, 95.8, 96.0, 94.0, 95.2),
        ];

        let signals = detect_patterns(&candles, Timeframe::H1, 95.2);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::DoubleTop)
            .expect("double top should be detected");
        assert_eq!(signal.direction, Direction::Short);
        assert_eq!(signal.status, PatternStatus::Holding);
        assert_eq!(signal.neckline, Some(96.0));
        assert!(signal.score >= 60);
    }

    #[test]
    fn detects_failed_high_sweep() {
        let candles = vec![
            candle_with_volume(0, 100.0, 105.0, 99.0, 104.0, 100.0),
            candle_with_volume(1, 104.0, 104.5, 100.0, 101.0, 100.0),
            candle_with_volume(2, 101.0, 106.8, 100.0, 102.0, 180.0),
            candle_with_volume(3, 102.0, 103.0, 98.5, 99.0, 110.0),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 99.0);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::SweepFailure)
            .expect("failed high sweep should be detected");
        assert_eq!(signal.direction, Direction::Short);
        assert_eq!(signal.status, PatternStatus::Holding);
        assert!(signal.score >= 55);
    }

    #[test]
    fn detects_failed_low_sweep() {
        let candles = vec![
            candle_with_volume(0, 100.0, 101.0, 95.0, 96.0, 100.0),
            candle_with_volume(1, 96.0, 100.0, 95.5, 99.0, 100.0),
            candle_with_volume(2, 99.0, 100.0, 93.2, 98.0, 180.0),
            candle_with_volume(3, 98.0, 101.5, 97.0, 101.0, 110.0),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 101.0);
        let signal = signals
            .iter()
            .find(|signal| {
                signal.kind == PatternKind::SweepFailure && signal.direction == Direction::Long
            })
            .expect("failed low sweep should be detected");

        assert_eq!(signal.status, PatternStatus::Holding);
        assert_eq!(signal.structure_score, 54);
        assert_eq!(signal.confirmation_score, 18);
        assert_eq!(signal.hold_score, 18);
        assert_eq!(signal.trade_score, 90);
        assert_eq!(signal.score, signal.trade_score);
    }

    #[test]
    fn double_top_requires_effective_neckline_break() {
        let candles = vec![
            candle(0, 100.0, 101.0, 99.0, 100.0),
            candle(1, 100.0, 106.0, 99.0, 105.0),
            candle(2, 105.0, 105.5, 100.5, 101.0),
            candle(3, 101.0, 102.0, 99.8, 100.2),
            candle(4, 100.2, 101.0, 99.7, 100.4),
            candle(5, 100.4, 101.2, 99.6, 100.1),
            candle(6, 100.1, 101.0, 99.5, 100.0),
            candle(7, 100.0, 102.0, 99.4, 101.5),
            candle(8, 101.5, 105.8, 100.8, 105.0),
            candle(9, 105.0, 105.2, 99.9, 100.05),
            candle(10, 100.05, 101.0, 99.7, 100.1),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 100.1);

        assert!(signals
            .iter()
            .filter(|signal| signal.kind == PatternKind::DoubleTop)
            .all(|signal| signal.status == PatternStatus::Forming));
    }

    #[test]
    fn double_bottom_requires_separated_pivots() {
        let candles = vec![
            candle(0, 101.0, 102.0, 100.0, 101.0),
            candle(1, 101.0, 102.0, 96.0, 97.0),
            candle(2, 97.0, 104.0, 97.0, 103.0),
            candle(3, 103.0, 104.0, 96.2, 98.0),
            candle(4, 98.0, 104.5, 98.0, 104.2),
            candle(5, 104.2, 105.0, 103.5, 104.8),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 104.8);

        assert!(signals
            .iter()
            .all(|signal| signal.kind != PatternKind::DoubleBottom));
    }

    #[test]
    fn holding_requires_follow_through_after_retest() {
        let candles = vec![
            candle(0, 100.0, 101.0, 99.0, 100.0),
            candle(1, 100.0, 101.0, 98.5, 99.5),
            candle(2, 99.5, 100.2, 98.2, 99.0),
            candle(3, 99.0, 100.0, 96.0, 97.0),
            candle(4, 97.0, 99.0, 96.8, 98.8),
            candle(5, 98.8, 101.0, 98.5, 100.0),
            candle(6, 100.0, 103.0, 99.5, 102.0),
            candle(7, 102.0, 104.0, 101.5, 103.5),
            candle(8, 103.5, 103.8, 101.0, 102.0),
            candle(9, 102.0, 102.5, 99.2, 100.0),
            candle(10, 100.0, 101.0, 97.2, 98.5),
            candle(11, 98.5, 100.0, 96.3, 97.8),
            candle(12, 97.8, 101.0, 97.5, 100.5),
            candle(13, 100.5, 105.2, 100.0, 104.8),
            candle(14, 104.8, 105.0, 103.8, 104.0),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 103.9);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::DoubleBottom)
            .expect("double bottom should be detected");

        assert_eq!(signal.status, PatternStatus::Retest);
    }

    #[test]
    fn pattern_exposes_structure_and_trade_scores() {
        let candles = vec![
            candle(0, 100.0, 101.0, 99.0, 100.0),
            candle(1, 100.0, 101.0, 98.5, 99.5),
            candle(2, 99.5, 100.2, 98.2, 99.0),
            candle(3, 99.0, 100.0, 96.0, 97.0),
            candle(4, 97.0, 99.0, 96.8, 98.8),
            candle(5, 98.8, 101.0, 98.5, 100.0),
            candle(6, 100.0, 103.0, 99.5, 102.0),
            candle(7, 102.0, 104.0, 101.5, 103.5),
            candle(8, 103.5, 103.8, 101.0, 102.0),
            candle(9, 102.0, 102.5, 99.2, 100.0),
            candle(10, 100.0, 101.0, 97.2, 98.5),
            candle(11, 98.5, 100.0, 96.3, 97.8),
            candle(12, 97.8, 101.0, 97.5, 100.5),
            candle(13, 100.5, 105.2, 100.0, 104.8),
            candle(14, 104.8, 105.0, 103.8, 104.0),
            candle(15, 104.0, 105.5, 103.9, 104.8),
        ];

        let signals = detect_patterns(&candles, Timeframe::M15, 104.8);
        let signal = signals
            .iter()
            .find(|signal| signal.kind == PatternKind::DoubleBottom)
            .expect("double bottom should be detected");

        assert!(signal.structure_score > 0);
        assert!(signal.confirmation_score > 0);
        assert!(signal.hold_score > 0);
        assert_eq!(
            signal.score,
            signal.structure_score + signal.confirmation_score + signal.hold_score
        );
        assert_eq!(signal.trade_score, signal.score);
    }
}
