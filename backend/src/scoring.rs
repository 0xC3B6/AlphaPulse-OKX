use crate::domain::{Direction, Score};

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

    Score {
        value: value.min(100),
        direction,
        reasons,
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
