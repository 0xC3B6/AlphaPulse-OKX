use std::collections::BTreeMap;

use crate::quality::add_tag;
pub use crate::quality::UniversePolicy;

const DAY_MS: f64 = 86_400_000.0;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentMetadata {
    pub inst_id: String,
    pub state: String,
    pub list_time_ms: i64,
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

pub fn build_filtered_symbol_universe(
    activity: &[MarketActivity],
    fixed_watchlist: &[String],
    dynamic_pool_size: usize,
    instruments: &[InstrumentMetadata],
    policy: UniversePolicy,
    now_ms: i64,
) -> Vec<UniverseSymbol> {
    let instrument_map: BTreeMap<_, _> = instruments
        .iter()
        .map(|instrument| (instrument.inst_id.as_str(), instrument))
        .collect();
    let mut ranked = activity.to_vec();
    ranked.sort_by(|left, right| {
        right
            .hotness_score()
            .partial_cmp(&left.hotness_score())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut symbols: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for item in ranked.iter() {
        if symbols
            .values()
            .filter(|tags| tags.iter().any(|tag| tag == "dynamic"))
            .count()
            >= dynamic_pool_size
        {
            break;
        }
        let Some(instrument) = instrument_map.get(item.inst_id.as_str()) else {
            continue;
        };
        if !instrument_is_monitorable(instrument, policy, now_ms) {
            continue;
        }

        let tags = symbols.entry(item.inst_id.clone()).or_default();
        add_tag(tags, "dynamic");
        add_listing_tags(tags, instrument, policy, now_ms);
    }

    for inst_id in fixed_watchlist {
        let Some(instrument) = instrument_map.get(inst_id.as_str()) else {
            continue;
        };
        if !instrument_is_monitorable(instrument, policy, now_ms) {
            continue;
        }

        let tags = symbols.entry(inst_id.clone()).or_default();
        add_tag(tags, "fixed");
        add_tag(tags, "manual_watch");
        add_listing_tags(tags, instrument, policy, now_ms);
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

fn instrument_is_monitorable(
    instrument: &InstrumentMetadata,
    policy: UniversePolicy,
    now_ms: i64,
) -> bool {
    instrument.state == "live"
        && listing_age_days(instrument, now_ms) >= policy.min_listing_age_days
}

fn add_listing_tags(
    tags: &mut Vec<String>,
    instrument: &InstrumentMetadata,
    policy: UniversePolicy,
    now_ms: i64,
) {
    if listing_age_days(instrument, now_ms) < policy.new_listing_days {
        add_tag(tags, "new_listing");
    }
}

fn listing_age_days(instrument: &InstrumentMetadata, now_ms: i64) -> f64 {
    ((now_ms - instrument.list_time_ms).max(0) as f64) / DAY_MS
}
