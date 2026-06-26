use alphapulse_okx_backend::universe::{
    build_filtered_symbol_universe, build_symbol_universe, InstrumentMetadata, MarketActivity,
    UniversePolicy,
};

const NOW_MS: i64 = 1782400000000;
const DAY_MS: i64 = 86_400_000;

#[test]
fn merges_dynamic_and_fixed_symbols_without_duplicates() {
    let activity = vec![
        MarketActivity::new("LAB-USDT-SWAP", 10_000_000.0, 0.12, 0.18, 0.20, 3.2),
        MarketActivity::new("BTC-USDT-SWAP", 50_000_000.0, 0.01, 0.02, 0.03, 1.1),
        MarketActivity::new("RAVE-USDT-SWAP", 8_000_000.0, -0.08, -0.16, 0.25, 2.8),
    ];
    let fixed = vec!["BTC-USDT-SWAP".to_string(), "DOGE-USDT-SWAP".to_string()];

    let universe = build_symbol_universe(&activity, &fixed, 2);

    assert_eq!(
        universe
            .iter()
            .filter(|symbol| symbol.inst_id == "BTC-USDT-SWAP")
            .count(),
        1
    );
    assert!(universe.iter().any(|symbol| symbol.inst_id == "DOGE-USDT-SWAP"
        && symbol.tags.contains(&"fixed".to_string())));
    assert!(universe.iter().any(|symbol| symbol.inst_id == "LAB-USDT-SWAP"
        && symbol.tags.contains(&"dynamic".to_string())));
}

#[test]
fn ranks_activity_by_hotness_score() {
    let activity = vec![
        MarketActivity::new("SLOW-USDT-SWAP", 100_000_000.0, 0.001, 0.002, 0.01, 1.0),
        MarketActivity::new("FAST-USDT-SWAP", 20_000_000.0, 0.10, 0.18, 0.22, 3.0),
    ];

    let universe = build_symbol_universe(&activity, &[], 1);

    assert_eq!(universe[0].inst_id, "FAST-USDT-SWAP");
}

#[test]
fn filters_pre_three_day_and_non_live_dynamic_symbols() {
    let activity = vec![
        MarketActivity::new("FRESH-USDT-SWAP", 100_000_000.0, 0.20, 0.25, 0.20, 3.0),
        MarketActivity::new("OLD-USDT-SWAP", 80_000_000.0, 0.04, 0.05, 0.08, 1.5),
        MarketActivity::new("PREOPEN-USDT-SWAP", 90_000_000.0, 0.12, 0.18, 0.22, 2.4),
    ];
    let instruments = vec![
        instrument("FRESH-USDT-SWAP", "live", 2),
        instrument("OLD-USDT-SWAP", "live", 30),
        instrument("PREOPEN-USDT-SWAP", "preopen", 30),
    ];

    let universe = build_filtered_symbol_universe(
        &activity,
        &[],
        10,
        &instruments,
        UniversePolicy::default(),
        NOW_MS,
    );

    assert_eq!(universe.len(), 1);
    assert_eq!(universe[0].inst_id, "OLD-USDT-SWAP");
}

#[test]
fn tags_new_listing_and_manual_watch_symbols() {
    let activity = vec![MarketActivity::new(
        "LAB-USDT-SWAP",
        100_000_000.0,
        0.04,
        0.05,
        0.08,
        1.5,
    )];
    let instruments = vec![instrument("LAB-USDT-SWAP", "live", 7)];
    let fixed = vec!["LAB-USDT-SWAP".to_string()];

    let universe = build_filtered_symbol_universe(
        &activity,
        &fixed,
        0,
        &instruments,
        UniversePolicy::default(),
        NOW_MS,
    );

    assert_eq!(universe.len(), 1);
    assert!(universe[0].tags.contains(&"fixed".to_string()));
    assert!(universe[0].tags.contains(&"manual_watch".to_string()));
    assert!(universe[0].tags.contains(&"new_listing".to_string()));
}

fn instrument(inst_id: &str, state: &str, age_days: i64) -> InstrumentMetadata {
    InstrumentMetadata {
        inst_id: inst_id.to_string(),
        state: state.to_string(),
        list_time_ms: NOW_MS - age_days * DAY_MS,
    }
}
