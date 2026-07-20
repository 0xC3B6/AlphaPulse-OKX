use chrono::{DateTime, Datelike, Duration, Timelike, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeTagKind {
    IntradayChaseLongRisk,
    IntradayChaseShortRisk,
    MultiDayChaseLongRisk,
    MultiDayChaseShortRisk,
    OverextensionReversalProbe,
    MiddayReversalWindow,
    MarketPenaltyApplied,
    TimeRiskAsiaOpen,
    TimeRiskMiddayReassessment,
    TimeRiskEuUsTransition,
    TimeRiskUsData,
    TimeRiskUsOpen,
    TimeRiskLateUs,
    TimeRiskWeekdayEvent,
    TimePenaltyApplied,
    RequiresHighConfidence,
    VwapExtensionRisk,
    AtrImpulseRisk,
    FundingCrowdingRisk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeTag {
    pub kind: TradeTagKind,
    pub label: String,
    pub score_impact: i32,
    pub reason: String,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeRegime {
    pub tags: Vec<TradeTag>,
    pub score_penalty: u8,
}

pub fn classify_time_regime(now_ms: i64) -> TimeRegime {
    let Some(now_utc) = DateTime::<Utc>::from_timestamp_millis(now_ms) else {
        return TimeRegime {
            tags: Vec::new(),
            score_penalty: 0,
        };
    };
    let china_time = now_utc + Duration::hours(8);
    let hour = china_time.hour();
    let minute = china_time.minute();
    let minutes = hour * 60 + minute;
    let mut tags = Vec::new();
    let mut score_penalty = 0_u8;

    if in_minutes(minutes, 8, 0, 9, 0) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskAsiaOpen,
            "asia open risk",
            4,
            "Asia liquidity pickup window",
        );
    }
    if in_minutes(minutes, 12, 0, 14, 0) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskMiddayReassessment,
            "midday reassessment",
            8,
            "midday reassessment window requires retest confirmation",
        );
    }
    if in_minutes(minutes, 18, 0, 20, 0) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskEuUsTransition,
            "EU/US transition risk",
            8,
            "EU/US transition can change liquidity conditions",
        );
    }
    if in_minutes(minutes, 20, 0, 21, 0) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskUsData,
            "US data risk",
            15,
            "US data window requires high-confidence structure",
        );
    }
    if in_minutes(minutes, 21, 0, 22, 0) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskUsOpen,
            "US equity open risk",
            18,
            "US equity open volatility requires high-confidence structure",
        );
    }
    if in_minutes(minutes, 2, 0, 4, 0) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskLateUs,
            "late US low-liquidity risk",
            12,
            "late US session can have thin liquidity and speech risk",
        );
    }
    if matches!(china_time.weekday(), Weekday::Wed | Weekday::Fri) {
        add_penalty(
            &mut tags,
            &mut score_penalty,
            now_ms,
            TradeTagKind::TimeRiskWeekdayEvent,
            "weekday event risk",
            4,
            "Wednesday or Friday event risk context",
        );
    }

    if score_penalty > 0 {
        tags.push(TradeTag {
            kind: TradeTagKind::TimePenaltyApplied,
            label: "time penalty applied".to_string(),
            score_impact: -(score_penalty as i32),
            reason: format!("time regime penalty {score_penalty}"),
            ts_ms: now_ms,
        });
    }
    if score_penalty >= 10 {
        tags.push(TradeTag {
            kind: TradeTagKind::RequiresHighConfidence,
            label: "requires high confidence".to_string(),
            score_impact: 0,
            reason: "time-risk window requires stronger confirmation".to_string(),
            ts_ms: now_ms,
        });
    }

    TimeRegime {
        tags,
        score_penalty,
    }
}

fn add_penalty(
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

fn in_minutes(
    minutes: u32,
    start_hour: u32,
    start_minute: u32,
    end_hour: u32,
    end_minute: u32,
) -> bool {
    let start = start_hour * 60 + start_minute;
    let end = end_hour * 60 + end_minute;
    minutes >= start && minutes < end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts_ms(raw: &str) -> i64 {
        chrono::DateTime::parse_from_rfc3339(raw)
            .unwrap()
            .timestamp_millis()
    }

    #[test]
    fn classifies_us_open_without_directional_bias() {
        let regime = classify_time_regime(ts_ms("2026-07-02T13:35:00Z"));

        assert!(regime
            .tags
            .iter()
            .any(|tag| tag.kind == TradeTagKind::TimeRiskUsOpen));
        assert!(regime
            .tags
            .iter()
            .any(|tag| tag.kind == TradeTagKind::RequiresHighConfidence));
        assert!(regime.score_penalty >= 15);
        assert!(regime.tags.iter().all(|tag| !tag.label.contains("short")));
        assert!(regime.tags.iter().all(|tag| !tag.label.contains("long")));
    }

    #[test]
    fn classifies_midday_as_reassessment_not_direction() {
        let regime = classify_time_regime(ts_ms("2026-07-02T04:30:00Z"));

        assert!(regime
            .tags
            .iter()
            .any(|tag| tag.kind == TradeTagKind::TimeRiskMiddayReassessment));
        assert!(regime.tags.iter().all(|tag| !tag.label.contains("short")));
        assert!(regime.tags.iter().all(|tag| !tag.label.contains("long")));
        assert!(regime.score_penalty >= 8);
    }
}
