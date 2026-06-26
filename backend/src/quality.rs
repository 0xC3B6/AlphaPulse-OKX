use crate::domain::Candle;

const DAY_MS: f64 = 86_400_000.0;

#[derive(Debug, Clone, Copy)]
pub struct UniversePolicy {
    pub min_listing_age_days: f64,
    pub new_listing_days: f64,
    pub min_history_days: f64,
    pub thin_history_days: f64,
}

impl Default for UniversePolicy {
    fn default() -> Self {
        Self {
            min_listing_age_days: 3.0,
            new_listing_days: 14.0,
            min_history_days: 3.0,
            thin_history_days: 7.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistoryDecision {
    pub allowed: bool,
    pub history_days: f64,
    pub tags: Vec<String>,
}

pub fn classify_history(candles: &[Candle], policy: UniversePolicy) -> HistoryDecision {
    let history_days = candle_history_days(candles);
    if history_days < policy.min_history_days {
        return HistoryDecision {
            allowed: false,
            history_days,
            tags: Vec::new(),
        };
    }

    let mut tags = Vec::new();
    if history_days < policy.thin_history_days {
        tags.push("thin_history".to_string());
    }

    HistoryDecision {
        allowed: true,
        history_days,
        tags,
    }
}

pub fn candle_history_days(candles: &[Candle]) -> f64 {
    let Some(first) = candles.first() else {
        return 0.0;
    };
    let Some(last) = candles.last() else {
        return 0.0;
    };
    ((last.ts_ms - first.ts_ms).max(0) as f64) / DAY_MS
}

pub fn add_tag(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|item| item == tag) {
        tags.push(tag.to_string());
    }
}
