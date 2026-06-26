use alphapulse_okx_backend::{
    domain::Direction,
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
