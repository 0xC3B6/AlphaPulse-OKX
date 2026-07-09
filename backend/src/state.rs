use std::{collections::BTreeMap, sync::Arc};

use chrono::Utc;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use crate::{
    domain::SymbolSnapshot,
    paper::{PaperAccountSnapshot, PaperError, PaperOrderRequest},
    persistence::PersistenceLayer,
    strategy::{
        StrategyCenterSnapshot, StrategyError, StrategyVersionSnapshot, VersionedPaperState,
        V3_VERSION_CODE,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub symbols: Vec<SymbolSnapshot>,
    pub last_scan_at_ms: Option<i64>,
    pub websocket_connected: bool,
    pub paper: PaperAccountSnapshot,
    pub strategy_center: StrategyCenterSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendEvent {
    Snapshot { data: DashboardSnapshot },
    SymbolUpdated { data: SymbolSnapshot },
    PaperUpdated { data: PaperAccountSnapshot },
    StrategyUpdated { data: StrategyCenterSnapshot },
}

#[derive(Clone)]
pub struct RadarState {
    inner: Arc<RwLock<RadarStateInner>>,
    events: broadcast::Sender<BackendEvent>,
    persistence: PersistenceLayer,
}

#[derive(Debug, Default)]
struct RadarStateInner {
    symbols: BTreeMap<String, SymbolSnapshot>,
    last_scan_at_ms: Option<i64>,
    websocket_connected: bool,
    paper: VersionedPaperState,
}

impl Default for RadarState {
    fn default() -> Self {
        Self::with_persistence(PersistenceLayer::disabled(), VersionedPaperState::default())
    }
}

impl RadarState {
    pub fn with_persistence(persistence: PersistenceLayer, paper: VersionedPaperState) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Arc::new(RwLock::new(RadarStateInner {
                paper,
                ..RadarStateInner::default()
            })),
            events,
            persistence,
        }
    }

    pub async fn snapshot(&self) -> DashboardSnapshot {
        let inner = self.inner.read().await;
        DashboardSnapshot {
            symbols: inner.symbols.values().cloned().collect(),
            last_scan_at_ms: inner.last_scan_at_ms,
            websocket_connected: inner.websocket_connected,
            paper: inner.paper.default_paper_snapshot(&inner.symbols),
            strategy_center: inner.paper.center_snapshot(&inner.symbols),
        }
    }

    pub async fn paper_snapshot(&self) -> PaperAccountSnapshot {
        let inner = self.inner.read().await;
        inner.paper.default_paper_snapshot(&inner.symbols)
    }

    pub async fn strategy_center_snapshot(&self) -> StrategyCenterSnapshot {
        let inner = self.inner.read().await;
        inner.paper.center_snapshot(&inner.symbols)
    }

    pub async fn strategy_version_snapshot(
        &self,
        version_code: &str,
    ) -> Result<StrategyVersionSnapshot, StrategyError> {
        let inner = self.inner.read().await;
        inner.paper.version_snapshot(version_code, &inner.symbols)
    }

    pub async fn upsert_symbol(&self, symbol: SymbolSnapshot) {
        let strategy_update = {
            let mut inner = self.inner.write().await;
            inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
            let prices = inner.symbols.clone();
            inner
                .paper
                .process_market_update(&symbol, &prices, symbol.updated_at_ms);
            inner.paper.center_snapshot(&inner.symbols)
        };
        let _ = self
            .events
            .send(BackendEvent::SymbolUpdated { data: symbol });
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: strategy_update,
        });
        self.persist_current_state("symbol_updated").await;
    }

    pub async fn upsert_symbol_without_strategy(&self, symbol: SymbolSnapshot) {
        {
            let mut inner = self.inner.write().await;
            inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
        }
        let _ = self
            .events
            .send(BackendEvent::SymbolUpdated { data: symbol });
        self.persist_current_state("symbol_updated").await;
    }

    pub async fn update_symbol_price(
        &self,
        inst_id: &str,
        price: f64,
        updated_at_ms: i64,
    ) -> Option<SymbolSnapshot> {
        let (updated, paper_update, strategy_update) = {
            let mut inner = self.inner.write().await;
            let symbol = inner.symbols.get_mut(inst_id)?;
            symbol.price = price;
            symbol.updated_at_ms = updated_at_ms;
            let updated = symbol.clone();
            let prices = inner.symbols.clone();
            inner
                .paper
                .process_market_update(&updated, &prices, updated_at_ms);
            let paper_update = inner.paper.default_paper_snapshot(&inner.symbols);
            let strategy_update = inner.paper.center_snapshot(&inner.symbols);
            (updated, Some(paper_update), strategy_update)
        };
        let _ = self.events.send(BackendEvent::SymbolUpdated {
            data: updated.clone(),
        });
        if let Some(paper) = paper_update {
            let _ = self.events.send(BackendEvent::PaperUpdated { data: paper });
        }
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: strategy_update,
        });
        self.persist_current_state("symbol_price_updated").await;
        Some(updated)
    }

    pub async fn open_paper_order(
        &self,
        order: PaperOrderRequest,
    ) -> Result<PaperAccountSnapshot, PaperError> {
        let (snapshot, strategy_snapshot) = {
            let mut inner = self.inner.write().await;
            let prices = inner.symbols.clone();
            inner
                .paper
                .open_order(
                    V3_VERSION_CODE,
                    order,
                    &prices,
                    Utc::now().timestamp_millis(),
                )
                .map_err(|error| match error {
                    StrategyError::Paper(error) => error,
                    StrategyError::UnknownVersion(version) => PaperError::PriceUnavailable(version),
                })?;
            (
                inner.paper.default_paper_snapshot(&inner.symbols),
                inner.paper.center_snapshot(&inner.symbols),
            )
        };
        let _ = self.events.send(BackendEvent::PaperUpdated {
            data: snapshot.clone(),
        });
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: strategy_snapshot,
        });
        self.persist_current_state("paper_order_opened").await;
        Ok(snapshot)
    }

    pub async fn close_paper_position(
        &self,
        inst_id: &str,
    ) -> Result<PaperAccountSnapshot, PaperError> {
        let (snapshot, strategy_snapshot) = {
            let mut inner = self.inner.write().await;
            let prices = inner.symbols.clone();
            inner
                .paper
                .close_position(
                    V3_VERSION_CODE,
                    inst_id,
                    &prices,
                    Utc::now().timestamp_millis(),
                )
                .map_err(|error| match error {
                    StrategyError::Paper(error) => error,
                    StrategyError::UnknownVersion(version) => PaperError::PriceUnavailable(version),
                })?;
            (
                inner.paper.default_paper_snapshot(&inner.symbols),
                inner.paper.center_snapshot(&inner.symbols),
            )
        };
        let _ = self.events.send(BackendEvent::PaperUpdated {
            data: snapshot.clone(),
        });
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: strategy_snapshot,
        });
        self.persist_current_state("paper_position_closed").await;
        Ok(snapshot)
    }

    pub async fn mark_scan(&self, ts_ms: i64) {
        let mut inner = self.inner.write().await;
        inner.last_scan_at_ms = Some(ts_ms);
        drop(inner);
        self.persist_current_state("scan_marked").await;
    }

    pub async fn start_strategy_version(
        &self,
        version_code: &str,
    ) -> Result<StrategyCenterSnapshot, StrategyError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            inner
                .paper
                .start_version(version_code, Utc::now().timestamp_millis())?;
            inner.paper.center_snapshot(&inner.symbols)
        };
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: snapshot.clone(),
        });
        self.persist_current_state("strategy_run_started").await;
        Ok(snapshot)
    }

    pub async fn stop_strategy_version(
        &self,
        version_code: &str,
    ) -> Result<StrategyCenterSnapshot, StrategyError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            inner
                .paper
                .stop_version(version_code, Utc::now().timestamp_millis())?;
            inner.paper.center_snapshot(&inner.symbols)
        };
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: snapshot.clone(),
        });
        self.persist_current_state("strategy_run_stopped").await;
        Ok(snapshot)
    }

    pub async fn reset_strategy_version(
        &self,
        version_code: &str,
    ) -> Result<StrategyCenterSnapshot, StrategyError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            inner
                .paper
                .reset_version(version_code, Utc::now().timestamp_millis())?;
            inner.paper.center_snapshot(&inner.symbols)
        };
        let _ = self.events.send(BackendEvent::StrategyUpdated {
            data: snapshot.clone(),
        });
        self.persist_current_state("strategy_run_reset").await;
        Ok(snapshot)
    }

    pub async fn set_websocket_connected(&self, connected: bool) {
        let mut inner = self.inner.write().await;
        inner.websocket_connected = connected;
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BackendEvent> {
        self.events.subscribe()
    }

    async fn persist_current_state(&self, event_type: &str) {
        if !self.persistence.is_postgres_enabled() && !self.persistence.is_redis_enabled() {
            return;
        }
        let (paper_state, snapshot) = {
            let inner = self.inner.read().await;
            (
                inner.paper.clone(),
                DashboardSnapshot {
                    symbols: inner.symbols.values().cloned().collect(),
                    last_scan_at_ms: inner.last_scan_at_ms,
                    websocket_connected: inner.websocket_connected,
                    paper: inner.paper.default_paper_snapshot(&inner.symbols),
                    strategy_center: inner.paper.center_snapshot(&inner.symbols),
                },
            )
        };
        if let Err(error) = self
            .persistence
            .persist_versioned_paper_state(&paper_state)
            .await
        {
            tracing::error!(?error, "failed to persist paper state");
        }
        if let Err(error) = self
            .persistence
            .persist_dashboard_snapshot(event_type, &snapshot)
            .await
        {
            tracing::error!(?error, "failed to persist dashboard snapshot");
        }
    }
}
