use std::collections::HashMap;

use crate::domain::{Direction, SymbolSnapshot};

#[derive(Debug, Clone, Copy)]
pub struct AlertThresholds {
    pub trend: u8,
    pub range: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertEvent {
    pub inst_id: String,
    pub kind: String,
    pub score: u8,
    pub direction: Direction,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AlertKey {
    direction: Direction,
    score_bucket: u8,
}

#[derive(Default)]
pub struct AlertTracker {
    last: HashMap<String, AlertKey>,
}

impl AlertTracker {
    pub fn evaluate(
        &mut self,
        snapshot: &SymbolSnapshot,
        thresholds: AlertThresholds,
    ) -> Vec<AlertEvent> {
        let mut events = Vec::new();
        self.maybe_push(
            snapshot,
            "trend",
            snapshot.trend_score.value,
            snapshot.trend_score.direction,
            thresholds.trend,
            &mut events,
        );
        self.maybe_push(
            snapshot,
            "range",
            snapshot.range_score.value,
            snapshot.range_score.direction,
            thresholds.range,
            &mut events,
        );
        events
    }

    fn maybe_push(
        &mut self,
        snapshot: &SymbolSnapshot,
        kind: &str,
        score: u8,
        direction: Direction,
        threshold: u8,
        events: &mut Vec<AlertEvent>,
    ) {
        if score < threshold || direction == Direction::Neutral {
            return;
        }

        let key_name = format!("{}:{kind}", snapshot.inst_id);
        let next = AlertKey {
            direction,
            score_bucket: score / 10,
        };

        if self.last.get(&key_name) == Some(&next) {
            return;
        }

        self.last.insert(key_name, next);
        events.push(AlertEvent {
            inst_id: snapshot.inst_id.clone(),
            kind: kind.to_string(),
            score,
            direction,
            message: format!(
                "{} {} {:?} {}: {}",
                snapshot.inst_id, kind, direction, score, snapshot.trigger_reason
            ),
        });
    }
}
