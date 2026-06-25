use alphapulse_okx_backend::universe::{build_symbol_universe, MarketActivity};

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
