use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};

use crate::{
    auto_strategy::{
        evaluate_auto_exit, evaluate_auto_strategy_at, AutoStrategyConfig, AutoStrategyDecision,
    },
    domain::SymbolSnapshot,
    paper::{
        PaperAccountSnapshot, PaperError, PaperOrderRequest, PaperState,
        SCALPING_OPTIMIZATION_SOURCE,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub symbols: Vec<SymbolSnapshot>,
    pub last_scan_at_ms: Option<i64>,
    pub websocket_connected: bool,
    pub paper: PaperAccountSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendEvent {
    Snapshot { data: DashboardSnapshot },
    SymbolUpdated { data: SymbolSnapshot },
    PaperUpdated { data: PaperAccountSnapshot },
}

#[derive(Clone)]
pub struct RadarState {
    inner: Arc<RwLock<RadarStateInner>>,
    events: broadcast::Sender<BackendEvent>,
}

#[derive(Debug, Default)]
struct RadarStateInner {
    symbols: BTreeMap<String, SymbolSnapshot>,
    latest_prices: BTreeMap<String, LatestPrice>,
    last_scan_at_ms: Option<i64>,
    websocket_connected: bool,
    paper: PaperState,
}

#[derive(Debug, Clone, Copy)]
struct LatestPrice {
    price: f64,
    updated_at_ms: i64,
}

impl RadarStateInner {
    fn price_for(&self, inst_id: &str) -> Option<f64> {
        self.latest_prices
            .get(inst_id)
            .map(|latest| latest.price)
            .or_else(|| self.symbols.get(inst_id).map(|symbol| symbol.price))
            .filter(|price| valid_price(*price))
    }

    fn price_map(&self) -> BTreeMap<String, f64> {
        let mut prices = BTreeMap::new();
        for (inst_id, symbol) in &self.symbols {
            if valid_price(symbol.price) {
                prices.insert(inst_id.clone(), symbol.price);
            }
        }
        for (inst_id, latest) in &self.latest_prices {
            if valid_price(latest.price) {
                prices.insert(inst_id.clone(), latest.price);
            }
        }
        prices
    }

    fn set_latest_price(&mut self, inst_id: &str, price: f64, updated_at_ms: i64) {
        if !valid_price(price) {
            return;
        }
        let should_update = self
            .latest_prices
            .get(inst_id)
            .map(|latest| updated_at_ms >= latest.updated_at_ms || latest.price != price)
            .unwrap_or(true);
        if should_update {
            self.latest_prices.insert(
                inst_id.to_string(),
                LatestPrice {
                    price,
                    updated_at_ms,
                },
            );
        }
    }
}

impl Default for RadarState {
    fn default() -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(RwLock::new(RadarStateInner::default())),
            events,
        }
    }
}

impl RadarState {
    pub async fn snapshot(&self) -> DashboardSnapshot {
        let inner = self.inner.read().await;
        let prices = inner.price_map();
        DashboardSnapshot {
            symbols: inner.symbols.values().cloned().collect(),
            last_scan_at_ms: inner.last_scan_at_ms,
            websocket_connected: inner.websocket_connected,
            paper: inner.paper.snapshot(&prices),
        }
    }

    pub async fn paper_snapshot(&self) -> PaperAccountSnapshot {
        let inner = self.inner.read().await;
        inner.paper.snapshot(&inner.price_map())
    }

    pub async fn upsert_symbol(&self, symbol: SymbolSnapshot) {
        {
            let mut inner = self.inner.write().await;
            inner.set_latest_price(&symbol.inst_id, symbol.price, symbol.updated_at_ms);
            inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
        }
        let _ = self
            .events
            .send(BackendEvent::SymbolUpdated { data: symbol });
    }

    pub async fn update_latest_prices(&self, prices: Vec<(String, f64, i64)>) {
        let (updated_symbols, paper_update) = {
            let mut inner = self.inner.write().await;
            let mut updated_ids = BTreeSet::new();
            let mut decision_ts_ms = Utc::now().timestamp_millis();
            let mut updated_symbols = Vec::new();

            for (inst_id, price, updated_at_ms) in prices {
                if !valid_price(price) {
                    continue;
                }
                decision_ts_ms = decision_ts_ms.max(updated_at_ms);
                updated_ids.insert(inst_id.clone());
                inner.set_latest_price(&inst_id, price, updated_at_ms);
                if let Some(symbol) = inner.symbols.get_mut(&inst_id) {
                    symbol.price = price;
                    symbol.updated_at_ms = updated_at_ms;
                    updated_symbols.push(symbol.clone());
                }
            }

            let mut changed_paper = false;
            for inst_id in inner.paper.open_position_inst_ids() {
                if updated_ids.contains(&inst_id) {
                    changed_paper |=
                        apply_auto_exit_for_inst_id(&mut inner, &inst_id, decision_ts_ms);
                }
            }

            for inst_id in &updated_ids {
                let Some(symbol) = inner.symbols.get(inst_id).cloned() else {
                    continue;
                };
                let price_map = inner.price_map();
                let paper = inner.paper.snapshot(&price_map);
                let Some(decision) = evaluate_auto_strategy_at(
                    &symbol,
                    &paper,
                    AutoStrategyConfig::default(),
                    decision_ts_ms,
                ) else {
                    continue;
                };
                match apply_strategy_decision(
                    &mut inner.paper,
                    &price_map,
                    decision,
                    decision_ts_ms,
                ) {
                    Ok(()) => changed_paper = true,
                    Err(error) => {
                        tracing::debug!(%inst_id, ?error, "automatic entry failed after price update")
                    }
                }
            }

            let paper_update = (changed_paper || inner.paper.has_open_positions())
                .then(|| inner.paper.snapshot(&inner.price_map()));
            (updated_symbols, paper_update)
        };

        for symbol in updated_symbols {
            let _ = self
                .events
                .send(BackendEvent::SymbolUpdated { data: symbol });
        }
        if let Some(paper) = paper_update {
            let _ = self.events.send(BackendEvent::PaperUpdated { data: paper });
        }
    }

    pub async fn ticker_sync_inst_ids(&self, fixed_watchlist: &[String]) -> Vec<String> {
        let inner = self.inner.read().await;
        let mut ids = BTreeSet::new();
        ids.extend(fixed_watchlist.iter().cloned());
        ids.extend(inner.paper.open_position_inst_ids());
        ids.into_iter().collect()
    }

    pub async fn update_symbol_price(
        &self,
        inst_id: &str,
        price: f64,
        updated_at_ms: i64,
    ) -> Option<SymbolSnapshot> {
        self.update_latest_prices(vec![(inst_id.to_string(), price, updated_at_ms)])
            .await;
        self.inner.read().await.symbols.get(inst_id).cloned()
    }

    pub async fn open_paper_order(
        &self,
        order: PaperOrderRequest,
    ) -> Result<PaperAccountSnapshot, PaperError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            let inst_id = order.inst_id.clone();
            let price = inner
                .price_for(&inst_id)
                .ok_or_else(|| PaperError::PriceUnavailable(inst_id.clone()))?;
            let prices = inner.price_map();
            let available_balance = inner.paper.snapshot(&prices).available_balance;
            inner.paper.open(
                order,
                price,
                available_balance,
                Utc::now().timestamp_millis(),
            )?;
            inner.paper.snapshot(&prices)
        };
        let _ = self.events.send(BackendEvent::PaperUpdated {
            data: snapshot.clone(),
        });
        Ok(snapshot)
    }

    pub async fn close_paper_position(
        &self,
        inst_id: &str,
    ) -> Result<PaperAccountSnapshot, PaperError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            let price = inner
                .price_for(inst_id)
                .ok_or_else(|| PaperError::PriceUnavailable(inst_id.to_string()))?;
            let prices = inner.price_map();
            inner
                .paper
                .close(inst_id, price, Utc::now().timestamp_millis())?;
            inner.paper.snapshot(&prices)
        };
        let _ = self.events.send(BackendEvent::PaperUpdated {
            data: snapshot.clone(),
        });
        Ok(snapshot)
    }

    pub async fn run_auto_strategy_for_symbol(
        &self,
        symbol: &SymbolSnapshot,
        config: AutoStrategyConfig,
    ) -> Result<Option<PaperAccountSnapshot>, PaperError> {
        self.run_auto_strategy_for_symbol_at(symbol, config, Utc::now().timestamp_millis())
            .await
    }

    pub async fn run_auto_strategy_for_symbol_at(
        &self,
        symbol: &SymbolSnapshot,
        config: AutoStrategyConfig,
        now_ms: i64,
    ) -> Result<Option<PaperAccountSnapshot>, PaperError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            inner.set_latest_price(&symbol.inst_id, symbol.price, symbol.updated_at_ms);
            let prices = inner.price_map();
            let paper = inner.paper.snapshot(&prices);
            let Some(decision) = evaluate_auto_strategy_at(symbol, &paper, config, now_ms) else {
                return Ok(None);
            };
            apply_strategy_decision(&mut inner.paper, &prices, decision, now_ms)?;
            inner.paper.snapshot(&prices)
        };
        let _ = self.events.send(BackendEvent::PaperUpdated {
            data: snapshot.clone(),
        });
        Ok(Some(snapshot))
    }

    pub async fn mark_scan(&self, ts_ms: i64) {
        let mut inner = self.inner.write().await;
        inner.last_scan_at_ms = Some(ts_ms);
    }

    pub async fn set_websocket_connected(&self, connected: bool) {
        let mut inner = self.inner.write().await;
        inner.websocket_connected = connected;
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BackendEvent> {
        self.events.subscribe()
    }
}

fn valid_price(price: f64) -> bool {
    price > 0.0 && price.is_finite()
}

fn apply_strategy_decision(
    paper: &mut PaperState,
    prices: &BTreeMap<String, f64>,
    decision: AutoStrategyDecision,
    now_ms: i64,
) -> Result<(), PaperError> {
    match decision {
        AutoStrategyDecision::Close {
            inst_id,
            reason,
            tags,
            execution_price,
            ..
        } => {
            let price = execution_price
                .filter(|price| valid_price(*price))
                .or_else(|| {
                    prices
                        .get(&inst_id)
                        .copied()
                        .filter(|price| valid_price(*price))
                })
                .ok_or_else(|| PaperError::PriceUnavailable(inst_id.clone()))?;
            paper.close_with_meta_and_tags(
                &inst_id,
                price,
                now_ms,
                SCALPING_OPTIMIZATION_SOURCE,
                &reason,
                tags,
            )?;
        }
        AutoStrategyDecision::Open {
            order,
            reason,
            tags,
        } => {
            let price = prices
                .get(&order.inst_id)
                .copied()
                .filter(|price| valid_price(*price))
                .ok_or_else(|| PaperError::PriceUnavailable(order.inst_id.clone()))?;
            let available_balance = paper.snapshot(prices).available_balance;
            paper.open_with_meta_and_tags(
                order,
                price,
                available_balance,
                now_ms,
                SCALPING_OPTIMIZATION_SOURCE,
                &reason,
                tags,
            )?;
        }
    }
    Ok(())
}

fn apply_auto_exit_for_inst_id(inner: &mut RadarStateInner, inst_id: &str, now_ms: i64) -> bool {
    let prices = inner.price_map();
    let paper = inner.paper.snapshot(&prices);
    let Some(decision) = evaluate_auto_exit(inst_id, &paper, AutoStrategyConfig::default()) else {
        return false;
    };
    match apply_strategy_decision(&mut inner.paper, &prices, decision, now_ms) {
        Ok(()) => true,
        Err(error) => {
            tracing::debug!(%inst_id, ?error, "automatic exit failed after price update");
            false
        }
    }
}
