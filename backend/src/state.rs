use std::{collections::BTreeMap, sync::Arc};

use chrono::Utc;
use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use crate::{
    domain::SymbolSnapshot,
    paper::{PaperAccountSnapshot, PaperError, PaperOrderRequest, PaperState},
};

#[derive(Debug, Clone, Serialize)]
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
    last_scan_at_ms: Option<i64>,
    websocket_connected: bool,
    paper: PaperState,
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
        DashboardSnapshot {
            symbols: inner.symbols.values().cloned().collect(),
            last_scan_at_ms: inner.last_scan_at_ms,
            websocket_connected: inner.websocket_connected,
            paper: inner.paper.snapshot(&inner.symbols),
        }
    }

    pub async fn paper_snapshot(&self) -> PaperAccountSnapshot {
        let inner = self.inner.read().await;
        inner.paper.snapshot(&inner.symbols)
    }

    pub async fn upsert_symbol(&self, symbol: SymbolSnapshot) {
        {
            let mut inner = self.inner.write().await;
            inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
        }
        let _ = self
            .events
            .send(BackendEvent::SymbolUpdated { data: symbol });
    }

    pub async fn update_symbol_price(
        &self,
        inst_id: &str,
        price: f64,
        updated_at_ms: i64,
    ) -> Option<SymbolSnapshot> {
        let (updated, paper_update) = {
            let mut inner = self.inner.write().await;
            let symbol = inner.symbols.get_mut(inst_id)?;
            symbol.price = price;
            symbol.updated_at_ms = updated_at_ms;
            let updated = symbol.clone();
            let paper_update = inner
                .paper
                .has_open_positions()
                .then(|| inner.paper.snapshot(&inner.symbols));
            (updated, paper_update)
        };
        let _ = self.events.send(BackendEvent::SymbolUpdated {
            data: updated.clone(),
        });
        if let Some(paper) = paper_update {
            let _ = self.events.send(BackendEvent::PaperUpdated { data: paper });
        }
        Some(updated)
    }

    pub async fn open_paper_order(
        &self,
        order: PaperOrderRequest,
    ) -> Result<PaperAccountSnapshot, PaperError> {
        let snapshot = {
            let mut inner = self.inner.write().await;
            let inst_id = order.inst_id.clone();
            let price = inner
                .symbols
                .get(&inst_id)
                .map(|symbol| symbol.price)
                .filter(|price| *price > 0.0 && price.is_finite())
                .ok_or_else(|| PaperError::PriceUnavailable(inst_id.clone()))?;
            let available_balance = inner.paper.snapshot(&inner.symbols).available_balance;
            inner.paper.open(
                order,
                price,
                available_balance,
                Utc::now().timestamp_millis(),
            )?;
            inner.paper.snapshot(&inner.symbols)
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
                .symbols
                .get(inst_id)
                .map(|symbol| symbol.price)
                .filter(|price| *price > 0.0 && price.is_finite())
                .ok_or_else(|| PaperError::PriceUnavailable(inst_id.to_string()))?;
            inner
                .paper
                .close(inst_id, price, Utc::now().timestamp_millis())?;
            inner.paper.snapshot(&inner.symbols)
        };
        let _ = self.events.send(BackendEvent::PaperUpdated {
            data: snapshot.clone(),
        });
        Ok(snapshot)
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
