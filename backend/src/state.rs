use std::{collections::BTreeMap, sync::Arc};

use serde::Serialize;
use tokio::sync::{broadcast, RwLock};

use crate::domain::SymbolSnapshot;

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSnapshot {
    pub symbols: Vec<SymbolSnapshot>,
    pub last_scan_at_ms: Option<i64>,
    pub websocket_connected: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendEvent {
    Snapshot { data: DashboardSnapshot },
    SymbolUpdated { data: SymbolSnapshot },
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
        }
    }

    pub async fn upsert_symbol(&self, symbol: SymbolSnapshot) {
        {
            let mut inner = self.inner.write().await;
            inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
        }
        let _ = self.events.send(BackendEvent::SymbolUpdated { data: symbol });
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
