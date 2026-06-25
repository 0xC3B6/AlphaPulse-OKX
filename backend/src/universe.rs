use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct MarketActivity {
    pub inst_id: String,
    pub quote_volume_24h: f64,
    pub change_15m_pct: f64,
    pub change_1h_pct: f64,
    pub volatility_1h_pct: f64,
    pub volume_ratio: f64,
}

impl MarketActivity {
    pub fn new(
        inst_id: &str,
        quote_volume_24h: f64,
        change_15m_pct: f64,
        change_1h_pct: f64,
        volatility_1h_pct: f64,
        volume_ratio: f64,
    ) -> Self {
        Self {
            inst_id: inst_id.to_string(),
            quote_volume_24h,
            change_15m_pct,
            change_1h_pct,
            volatility_1h_pct,
            volume_ratio,
        }
    }

    pub fn hotness_score(&self) -> f64 {
        let volume_component = (self.quote_volume_24h.max(1.0).log10() / 10.0).min(1.0) * 20.0;
        let movement_component = (self.change_15m_pct.abs() * 160.0).min(25.0)
            + (self.change_1h_pct.abs() * 100.0).min(20.0);
        let volatility_component = (self.volatility_1h_pct * 100.0).min(20.0);
        let volume_ratio_component = ((self.volume_ratio - 1.0).max(0.0) * 8.0).min(15.0);
        volume_component + movement_component + volatility_component + volume_ratio_component
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniverseSymbol {
    pub inst_id: String,
    pub tags: Vec<String>,
}

pub fn build_symbol_universe(
    activity: &[MarketActivity],
    fixed_watchlist: &[String],
    dynamic_pool_size: usize,
) -> Vec<UniverseSymbol> {
    let mut ranked = activity.to_vec();
    ranked.sort_by(|left, right| {
        right
            .hotness_score()
            .partial_cmp(&left.hotness_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut symbols: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for item in ranked.into_iter().take(dynamic_pool_size) {
        symbols
            .entry(item.inst_id)
            .or_default()
            .push("dynamic".to_string());
    }

    for inst_id in fixed_watchlist {
        symbols
            .entry(inst_id.clone())
            .or_default()
            .push("fixed".to_string());
    }

    let mut output: Vec<UniverseSymbol> = symbols
        .into_iter()
        .map(|(inst_id, mut tags)| {
            tags.sort();
            tags.dedup();
            UniverseSymbol { inst_id, tags }
        })
        .collect();

    output.sort_by(|left, right| {
        let left_score = activity
            .iter()
            .find(|item| item.inst_id == left.inst_id)
            .map(MarketActivity::hotness_score)
            .unwrap_or(0.0);
        let right_score = activity
            .iter()
            .find(|item| item.inst_id == right.inst_id)
            .map(MarketActivity::hotness_score)
            .unwrap_or(0.0);
        right_score
            .partial_cmp(&left_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    output
}
