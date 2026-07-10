use alphapulse_okx_backend::{
    domain::{Direction, PatternKind, PatternLevelZone, PatternSignal, PatternStatus, Timeframe},
    scoring::{score_symbol, ScoringInput},
};

#[test]
fn scores_strong_short_trend_with_explanations() {
    let input = ScoringInput {
        inst_id: "LAB-USDT-SWAP".to_string(),
        change_5m_pct: -0.045,
        change_15m_pct: -0.082,
        change_1h_pct: -0.12,
        broke_recent_high: false,
        broke_recent_low: true,
        volume_ratio: 3.1,
        nearest_fvg_distance_pct: Some(0.014),
        dynamic_pool: true,
        near_support: false,
        near_resistance: false,
        clear_range: false,
        funding_rate: Some(-0.003),
        pattern_signals: vec![],
    };

    let scored = score_symbol(input);

    assert!(scored.trend_score.value >= 80);
    assert_eq!(scored.trend_score.direction, Direction::Short);
    assert!(scored
        .trend_score
        .reasons
        .iter()
        .any(|reason| reason.contains("15m move")));
    assert!(scored
        .trend_score
        .reasons
        .iter()
        .any(|reason| reason.contains("volume")));
}

#[test]
fn scores_range_short_near_resistance() {
    let input = ScoringInput {
        inst_id: "LAB-USDT-SWAP".to_string(),
        change_5m_pct: 0.012,
        change_15m_pct: 0.018,
        change_1h_pct: 0.021,
        broke_recent_high: false,
        broke_recent_low: false,
        volume_ratio: 2.4,
        nearest_fvg_distance_pct: Some(0.006),
        dynamic_pool: true,
        near_support: false,
        near_resistance: true,
        clear_range: true,
        funding_rate: Some(0.002),
        pattern_signals: vec![],
    };

    let scored = score_symbol(input);

    assert!(scored.range_score.value >= 80);
    assert_eq!(scored.range_score.direction, Direction::Short);
    assert!(scored
        .range_score
        .reasons
        .iter()
        .any(|reason| reason.contains("resistance")));
}

#[test]
fn confirmed_pattern_does_not_override_neutral_trend_direction() {
    let input = ScoringInput {
        inst_id: "ETH-USDT-SWAP".to_string(),
        change_5m_pct: 0.001,
        change_15m_pct: 0.004,
        change_1h_pct: 0.006,
        broke_recent_high: false,
        broke_recent_low: false,
        volume_ratio: 1.8,
        nearest_fvg_distance_pct: None,
        dynamic_pool: false,
        near_support: false,
        near_resistance: false,
        clear_range: false,
        funding_rate: None,
        pattern_signals: vec![PatternSignal {
            kind: PatternKind::DoubleBottom,
            direction: Direction::Long,
            timeframe: Timeframe::M15,
            status: PatternStatus::Holding,
            score: 92,
            structure_score: 52,
            confirmation_score: 20,
            hold_score: 20,
            trade_score: 92,
            neckline: Some(1_600.0),
            invalidation_level: Some(1_540.0),
            start_ts_ms: 1,
            confirm_ts_ms: Some(2),
            pivots: Vec::new(),
            level_zone: Some(PatternLevelZone {
                lower: 1_590.0,
                upper: 1_610.0,
            }),
            reasons: vec!["test pattern".to_string()],
            warnings: Vec::new(),
        }],
    };

    let scored = score_symbol(input);

    assert_eq!(scored.trend_score.direction, Direction::Neutral);
    assert_eq!(scored.trend_score.value, 0);
}
