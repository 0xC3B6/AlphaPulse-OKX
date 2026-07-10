use crate::domain::{Direction, PatternKind, PatternSignal, PatternStatus, Score};

#[derive(Debug, Clone)]
pub struct ScoringInput {
    pub inst_id: String,
    pub change_5m_pct: f64,
    pub change_15m_pct: f64,
    pub change_1h_pct: f64,
    pub broke_recent_high: bool,
    pub broke_recent_low: bool,
    pub volume_ratio: f64,
    pub nearest_fvg_distance_pct: Option<f64>,
    pub dynamic_pool: bool,
    pub near_support: bool,
    pub near_resistance: bool,
    pub clear_range: bool,
    pub funding_rate: Option<f64>,
    pub pattern_signals: Vec<PatternSignal>,
}

#[derive(Debug, Clone)]
pub struct ScoredSymbol {
    pub trend_score: Score,
    pub range_score: Score,
}

pub fn score_symbol(input: ScoringInput) -> ScoredSymbol {
    ScoredSymbol {
        trend_score: trend_score(&input),
        range_score: range_score(&input),
    }
}

fn trend_score(input: &ScoringInput) -> Score {
    let mut value = 0_u8;
    let mut reasons = Vec::new();
    let direction = if input.change_15m_pct < -0.025 && input.change_1h_pct < -0.03 {
        Direction::Short
    } else if input.change_15m_pct > 0.025 && input.change_1h_pct > 0.03 {
        Direction::Long
    } else {
        Direction::Neutral
    };

    if direction != Direction::Neutral {
        value += 25;
        reasons.push(format!(
            "15m move {:.1}% aligns with 1h move {:.1}%",
            input.change_15m_pct * 100.0,
            input.change_1h_pct * 100.0
        ));
    }
    if input.broke_recent_low && direction == Direction::Short {
        value += 20;
        reasons.push("broke recent low".to_string());
    }
    if input.broke_recent_high && direction == Direction::Long {
        value += 20;
        reasons.push("broke recent high".to_string());
    }
    if input.volume_ratio >= 2.0 {
        value += 20;
        reasons.push(format!("volume {:.1}x", input.volume_ratio));
    }
    if input
        .nearest_fvg_distance_pct
        .is_some_and(|distance| distance <= 0.02)
    {
        value += 15;
        reasons.push("near FVG zone".to_string());
    }
    if input.dynamic_pool {
        value += 10;
        reasons.push("in dynamic hot pool".to_string());
    }
    if input.funding_rate.is_some_and(|rate| rate.abs() >= 0.002) {
        value += 5;
        reasons.push("funding rate is elevated".to_string());
    }
    if let Some((pattern_direction, boost, reason)) = best_pattern_boost(input) {
        if direction != Direction::Neutral && direction == pattern_direction {
            value = value.saturating_add(boost);
            reasons.push(reason);
        }
    }

    Score {
        value: value.min(100),
        direction,
        reasons,
    }
}

fn best_pattern_boost(input: &ScoringInput) -> Option<(Direction, u8, String)> {
    input
        .pattern_signals
        .iter()
        .filter(|signal| {
            matches!(
                signal.status,
                PatternStatus::Confirmed | PatternStatus::Retest | PatternStatus::Holding
            ) && signal.trade_score >= 75
        })
        .filter_map(|signal| {
            let status_cap = match signal.status {
                PatternStatus::Retest => 26,
                PatternStatus::Holding => 22,
                PatternStatus::Confirmed => 12,
                PatternStatus::Forming | PatternStatus::Invalidated => return None,
            };
            let boost = ((signal.trade_score as f64 / 100.0) * status_cap as f64).round() as u8;
            Some((
                signal.direction,
                boost.max(8),
                format!(
                    "pattern {} {} {}",
                    pattern_kind_label(signal.kind),
                    pattern_status_label(signal.status),
                    signal.trade_score
                ),
            ))
        })
        .max_by_key(|(_, boost, _)| *boost)
}

fn pattern_kind_label(kind: PatternKind) -> &'static str {
    match kind {
        PatternKind::DoubleBottom => "double bottom",
        PatternKind::DoubleTop => "double top",
        PatternKind::SweepFailure => "sweep failure",
    }
}

fn pattern_status_label(status: PatternStatus) -> &'static str {
    match status {
        PatternStatus::Forming => "forming",
        PatternStatus::Confirmed => "confirmed",
        PatternStatus::Retest => "retest",
        PatternStatus::Holding => "holding",
        PatternStatus::Invalidated => "invalidated",
    }
}

fn range_score(input: &ScoringInput) -> Score {
    let mut value = 0_u8;
    let mut reasons = Vec::new();
    let mut direction = Direction::Neutral;

    if input.clear_range {
        value += 25;
        reasons.push("clear recent range".to_string());
    }
    if input.near_resistance {
        value += 25;
        direction = Direction::Short;
        reasons.push("near resistance".to_string());
    }
    if input.near_support {
        value += 25;
        direction = Direction::Long;
        reasons.push("near support".to_string());
    }
    if input.volume_ratio >= 2.0 {
        value += 20;
        reasons.push(format!("boundary volume {:.1}x", input.volume_ratio));
    }
    if input
        .nearest_fvg_distance_pct
        .is_some_and(|distance| distance <= 0.01)
    {
        value += 15;
        reasons.push("FVG overlaps nearby area".to_string());
    }
    if input.funding_rate.is_some_and(|rate| rate.abs() >= 0.002) {
        value += 10;
        reasons.push("funding rate supports caution".to_string());
    }
    if input.change_15m_pct.abs() > 0.10 {
        value = value.saturating_sub(15);
        reasons.push("very high 15m movement reduces range quality".to_string());
    }

    Score {
        value: value.min(100),
        direction,
        reasons,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PatternKind, PatternSignal, PatternStatus, Timeframe};

    fn pattern(
        kind: PatternKind,
        direction: Direction,
        status: PatternStatus,
        score: u8,
    ) -> PatternSignal {
        PatternSignal {
            kind,
            direction,
            timeframe: Timeframe::M15,
            status,
            score,
            structure_score: score.saturating_sub(40),
            confirmation_score: 20,
            hold_score: 20,
            trade_score: score,
            neckline: Some(104.0),
            invalidation_level: Some(98.0),
            start_ts_ms: 0,
            confirm_ts_ms: Some(1),
            pivots: vec![],
            level_zone: None,
            reasons: vec!["test pattern".to_string()],
            warnings: vec![],
        }
    }

    fn base_input(pattern_signals: Vec<PatternSignal>) -> ScoringInput {
        ScoringInput {
            inst_id: "ETH-USDT-SWAP".to_string(),
            change_5m_pct: 0.0,
            change_15m_pct: 0.0,
            change_1h_pct: 0.0,
            broke_recent_high: false,
            broke_recent_low: false,
            volume_ratio: 1.0,
            nearest_fvg_distance_pct: None,
            dynamic_pool: false,
            near_support: false,
            near_resistance: false,
            clear_range: false,
            funding_rate: None,
            pattern_signals,
        }
    }

    #[test]
    fn w_bottom_holding_boosts_existing_long_trend() {
        let mut input = base_input(vec![pattern(
            PatternKind::DoubleBottom,
            Direction::Long,
            PatternStatus::Holding,
            92,
        )]);
        input.change_15m_pct = 0.03;
        input.change_1h_pct = 0.04;

        let scored = score_symbol(input);

        assert_eq!(scored.trend_score.direction, Direction::Long);
        assert!(scored.trend_score.value > 25);
        assert!(scored
            .trend_score
            .reasons
            .iter()
            .any(|reason| reason.contains("pattern")));
    }

    #[test]
    fn forming_pattern_does_not_boost_score() {
        let scored = score_symbol(base_input(vec![pattern(
            PatternKind::DoubleBottom,
            Direction::Long,
            PatternStatus::Forming,
            80,
        )]));

        assert_eq!(scored.trend_score.direction, Direction::Neutral);
        assert_eq!(scored.trend_score.value, 0);
    }

    #[test]
    fn invalidated_pattern_does_not_boost_existing_long_trend() {
        let mut input = base_input(vec![pattern(
            PatternKind::DoubleBottom,
            Direction::Long,
            PatternStatus::Invalidated,
            100,
        )]);
        input.change_15m_pct = 0.03;
        input.change_1h_pct = 0.04;

        let scored = score_symbol(input);

        assert_eq!(scored.trend_score.direction, Direction::Long);
        assert_eq!(scored.trend_score.value, 25);
        assert!(!scored
            .trend_score
            .reasons
            .iter()
            .any(|reason| reason.contains("pattern")));
    }
}
