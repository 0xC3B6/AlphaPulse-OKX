use alphapulse_okx_backend::{
    alerts::{AlertThresholds, AlertTracker},
    domain::{Direction, ScalpingMetrics, Score, SymbolSnapshot},
};

fn snapshot(inst_id: &str, trend: u8, range: u8, direction: Direction) -> SymbolSnapshot {
    SymbolSnapshot {
        inst_id: inst_id.to_string(),
        price: 17.2,
        change_5m_pct: -0.03,
        change_15m_pct: -0.07,
        change_1h_pct: -0.11,
        amplitude_24h_pct: 0.0,
        trend_score: Score {
            value: trend,
            direction,
            reasons: vec!["volume 3.1x".to_string()],
        },
        range_score: Score {
            value: range,
            direction: Direction::Neutral,
            reasons: vec![],
        },
        pool_tags: vec!["dynamic".to_string()],
        trigger_reason: "trend short 84: volume 3.1x".to_string(),
        funding_rate: Some(-0.003),
        scalping_metrics: ScalpingMetrics::default(),
        fvgs: vec![],
        levels: vec![],
        pattern_signals: vec![],
        updated_at_ms: 1_782_400_000_000,
    }
}

#[test]
fn alerts_only_when_symbol_newly_enters_high_score_state() {
    let mut tracker = AlertTracker::default();
    let thresholds = AlertThresholds {
        trend: 80,
        range: 80,
    };

    let first = tracker.evaluate(
        &snapshot("LAB-USDT-SWAP", 84, 20, Direction::Short),
        thresholds,
    );
    let second = tracker.evaluate(
        &snapshot("LAB-USDT-SWAP", 84, 20, Direction::Short),
        thresholds,
    );

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
}

#[test]
fn re_alerts_when_direction_changes() {
    let mut tracker = AlertTracker::default();
    let thresholds = AlertThresholds {
        trend: 80,
        range: 80,
    };

    let _ = tracker.evaluate(
        &snapshot("LAB-USDT-SWAP", 84, 20, Direction::Short),
        thresholds,
    );
    let changed = tracker.evaluate(
        &snapshot("LAB-USDT-SWAP", 86, 20, Direction::Long),
        thresholds,
    );

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].direction, Direction::Long);
}
