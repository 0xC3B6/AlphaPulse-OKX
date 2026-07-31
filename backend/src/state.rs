use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, Mutex, RwLock};

use crate::{
    auto_strategy::{
        evaluate_auto_exit, evaluate_auto_strategy_observed_at, AutoStrategyConfig,
        AutoStrategyDecision,
    },
    domain::SymbolSnapshot,
    observability::{PositionContextEvent, PositionContextEventKind, StrategyCandidateEvent},
    paper::{
        append_equity_candles, PaperAccountSnapshot, PaperEquityCandle, PaperEquityCurves,
        PaperEquityPoint, PaperError, PaperOrderRequest, PaperSide, PaperState,
        SCALPING_OPTIMIZATION_SOURCE,
    },
    persistence::{
        PersistedOrderIntent, PersistedTransition, PersistenceHealthSnapshot, PersistenceLayer,
        PersistenceStatus,
    },
    risk_safety::{
        AccountEvent, AccountEventEnvelope, AccountEventResult, AccountRiskSnapshot,
        AccountRiskState,
    },
    strategy_identity::{StrategyIdentity, StrategyRunMode},
    strategy_runtime::{StrategyRuntime, StrategyRuntimeContainer, StrategyRuntimeError},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSnapshot {
    pub symbols: Vec<SymbolSnapshot>,
    pub last_scan_at_ms: Option<i64>,
    pub websocket_connected: bool,
    pub paper: PaperAccountSnapshot,
    pub persistence: PersistenceHealthSnapshot,
    pub risk: AccountRiskSnapshot,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BackendEvent {
    Snapshot { data: Box<DashboardSnapshot> },
    SymbolUpdated { data: Box<SymbolSnapshot> },
    PaperUpdated { data: Box<PaperAccountSnapshot> },
    StrategyRunUpdated { data: Box<PaperAccountSnapshot> },
}

#[derive(Debug, thiserror::Error)]
pub enum PaperTransitionError {
    #[error(transparent)]
    Paper(#[from] PaperError),
    #[error("paper persistence is paused: {0}")]
    PersistencePaused(String),
    #[error("paper persistence failed: {0}")]
    Persistence(String),
    #[error("account is close-only: {0}")]
    RiskCloseOnly(String),
    #[error(transparent)]
    StrategyRuntime(#[from] StrategyRuntimeError),
}

impl PaperTransitionError {
    fn persistence(error: impl std::fmt::Display) -> Self {
        Self::Persistence(error.to_string())
    }
}

#[derive(Clone)]
pub struct RadarState {
    inner: Arc<RwLock<RadarStateInner>>,
    events: broadcast::Sender<BackendEvent>,
    persistence: Option<PersistenceLayer>,
    // Tokio's mutex is FIFO. Holding it through mutation, PostgreSQL commit and
    // in-memory publication makes this the single per-account event queue.
    account_event_queue: Arc<Mutex<()>>,
}

#[derive(Debug)]
struct RadarStateInner {
    symbols: BTreeMap<String, SymbolSnapshot>,
    latest_prices: BTreeMap<String, LatestPrice>,
    last_scan_at_ms: Option<i64>,
    websocket_connected: bool,
    strategy_runs: StrategyRuntimeContainer,
    persistence: PersistenceHealthSnapshot,
    risk: AccountRiskState,
    risk_event_sequence: u64,
}

#[derive(Debug, Clone, Copy)]
struct LatestPrice {
    price: f64,
    updated_at_ms: i64,
}

fn equity_candle_close_point(candle: &PaperEquityCandle) -> PaperEquityPoint {
    PaperEquityPoint {
        timestamp_ms: candle.bucket_start_ms,
        equity: candle.close_equity,
        realized_pnl: candle.realized_pnl,
        unrealized_pnl: candle.unrealized_pnl,
        open_positions_count: candle.open_positions_count,
    }
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

    fn dashboard(&self) -> DashboardSnapshot {
        DashboardSnapshot {
            symbols: self.symbols.values().cloned().collect(),
            last_scan_at_ms: self.last_scan_at_ms,
            websocket_connected: self.websocket_connected,
            paper: self.paper_snapshot(),
            persistence: self.persistence.clone(),
            risk: self.risk.snapshot(),
        }
    }

    fn paper_snapshot(&self) -> PaperAccountSnapshot {
        self.paper_snapshot_for_run(self.strategy_runs.active_run_id())
            .expect("active strategy runtime exists")
    }

    fn paper_snapshot_for_run(&self, run_id: &str) -> Option<PaperAccountSnapshot> {
        let runtime = self.strategy_runs.get(run_id).ok()?;
        let mut snapshot = runtime.paper().snapshot(&self.price_map());
        snapshot.persistence = self.persistence.clone();
        let mut equity_curves = runtime.equity_curves().clone();
        if let Some(timestamp_ms) = self
            .latest_prices
            .values()
            .map(|latest| latest.updated_at_ms)
            .max()
        {
            append_equity_candles(
                &mut equity_curves,
                PaperEquityPoint::from_snapshot(timestamp_ms, &snapshot),
            );
        }
        snapshot.equity_history = equity_curves
            .get("all")
            .into_iter()
            .flatten()
            .map(equity_candle_close_point)
            .collect();
        snapshot.equity_curves = equity_curves;
        Some(snapshot)
    }

    fn set_latest_price(&mut self, inst_id: &str, price: f64, updated_at_ms: i64) -> bool {
        if !valid_price(price) {
            return false;
        }
        let should_update = self
            .latest_prices
            .get(inst_id)
            .map(|latest| updated_at_ms > latest.updated_at_ms)
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
        should_update
    }

    fn next_risk_event_sequence(&mut self) -> u64 {
        self.risk_event_sequence = self.risk_event_sequence.saturating_add(1);
        self.risk_event_sequence
    }
}

impl Default for RadarState {
    fn default() -> Self {
        Self::new(None, PaperState::default(), PaperEquityCurves::new())
    }
}

impl RadarState {
    pub fn with_persistence(persistence: PersistenceLayer, paper: PaperState) -> Self {
        Self::new(Some(persistence), paper, PaperEquityCurves::new())
    }

    pub fn with_persistence_and_equity_curves(
        persistence: PersistenceLayer,
        paper: PaperState,
        equity_curves: PaperEquityCurves,
    ) -> Self {
        Self::new(Some(persistence), paper, equity_curves)
    }

    pub fn with_persistence_and_strategy_runs(
        persistence: PersistenceLayer,
        strategy_runs: StrategyRuntimeContainer,
    ) -> Self {
        Self::new_with_strategy_runs(Some(persistence), strategy_runs)
    }

    fn new(
        persistence: Option<PersistenceLayer>,
        paper: PaperState,
        equity_curves: PaperEquityCurves,
    ) -> Self {
        let active_runtime =
            StrategyRuntime::new(paper, AutoStrategyConfig::default(), equity_curves)
                .expect("active strategy identity matches restored V3 config");
        let strategy_runs = StrategyRuntimeContainer::new(active_runtime)
            .expect("restored V3 is the active strategy runtime");
        Self::new_with_strategy_runs(persistence, strategy_runs)
    }

    fn new_with_strategy_runs(
        persistence: Option<PersistenceLayer>,
        strategy_runs: StrategyRuntimeContainer,
    ) -> Self {
        let (events, _) = broadcast::channel(256);
        let has_positions = !strategy_runs
            .active()
            .paper()
            .open_position_inst_ids()
            .is_empty();
        let scope = persistence
            .as_ref()
            .map(|layer| layer.account_scope().clone())
            .unwrap_or_default();
        Self {
            inner: Arc::new(RwLock::new(RadarStateInner {
                strategy_runs,
                symbols: BTreeMap::new(),
                latest_prices: BTreeMap::new(),
                last_scan_at_ms: None,
                websocket_connected: false,
                persistence: PersistenceHealthSnapshot::default(),
                risk: AccountRiskState::startup(scope, has_positions),
                risk_event_sequence: 0,
            })),
            events,
            persistence,
            account_event_queue: Arc::new(Mutex::new(())),
        }
    }

    pub async fn snapshot(&self) -> DashboardSnapshot {
        self.inner.read().await.dashboard()
    }

    pub async fn paper_snapshot(&self) -> PaperAccountSnapshot {
        self.inner.read().await.paper_snapshot()
    }

    pub async fn strategy_run_snapshot(
        &self,
        run_id: &str,
    ) -> Result<PaperAccountSnapshot, PaperTransitionError> {
        self.inner
            .read()
            .await
            .paper_snapshot_for_run(run_id)
            .ok_or_else(|| StrategyRuntimeError::RunNotFound(run_id.to_string()).into())
    }

    pub async fn strategy_run_snapshots(&self) -> Vec<PaperAccountSnapshot> {
        let inner = self.inner.read().await;
        inner
            .strategy_runs
            .ordered_run_ids()
            .iter()
            .filter_map(|run_id| inner.paper_snapshot_for_run(run_id))
            .collect()
    }

    pub async fn register_shadow_strategy(
        &self,
        identity: StrategyIdentity,
        run_id: impl Into<String>,
        config: AutoStrategyConfig,
    ) -> Result<PaperAccountSnapshot, PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        let paper = PaperState::fresh_isolated(identity, run_id, StrategyRunMode::ShadowPaper)?;
        let runtime = StrategyRuntime::new(paper.clone(), config, PaperEquityCurves::new())?;
        {
            self.inner
                .read()
                .await
                .strategy_runs
                .validate_shadow(&runtime)?;
        }
        let prices = self.inner.read().await.price_map();
        let snapshot = paper.snapshot(&prices);
        if let Some(persistence) = &self.persistence {
            if let Err(error) = persistence
                .persist_strategy_run_registration(
                    &paper,
                    &snapshot,
                    &config,
                    Utc::now().timestamp_millis(),
                )
                .await
            {
                self.pause_persistence(error.to_string()).await;
                return Err(PaperTransitionError::persistence(error));
            }
        }
        let run_id = paper.run_id().to_string();
        let mut inner = self.inner.write().await;
        inner.strategy_runs.register_shadow(runtime)?;
        inner
            .paper_snapshot_for_run(&run_id)
            .ok_or_else(|| StrategyRuntimeError::RunNotFound(run_id).into())
    }

    pub async fn persistence_health(&self) -> PersistenceHealthSnapshot {
        self.inner.read().await.persistence.clone()
    }

    pub async fn handle_account_event(&self, envelope: AccountEventEnvelope) -> AccountEventResult {
        let _transition = self.account_event_queue.lock().await;
        let (result, dashboard) = {
            let mut inner = self.inner.write().await;
            let result = inner.risk.handle(envelope);
            (result, inner.dashboard())
        };
        let _ = self.events.send(BackendEvent::Snapshot {
            data: Box::new(dashboard),
        });
        result
    }

    pub async fn upsert_symbol(&self, mut symbol: SymbolSnapshot) {
        let accepted = {
            let mut inner = self.inner.write().await;
            if !valid_price(symbol.price) {
                false
            } else {
                let latest = inner.latest_prices.get(&symbol.inst_id).copied();
                match latest {
                    Some(latest) if symbol.updated_at_ms < latest.updated_at_ms => false,
                    Some(latest) if symbol.updated_at_ms == latest.updated_at_ms => {
                        // A scan seeds latest_prices from the REST ticker before it builds the
                        // richer radar snapshot from that same ticker. Publish the analysis
                        // snapshot without treating the duplicate timestamp as a new price event.
                        symbol.price = latest.price;
                        inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
                        true
                    }
                    _ => {
                        inner.set_latest_price(&symbol.inst_id, symbol.price, symbol.updated_at_ms);
                        inner.symbols.insert(symbol.inst_id.clone(), symbol.clone());
                        true
                    }
                }
            }
        };
        if !accepted {
            return;
        }
        let _ = self.events.send(BackendEvent::SymbolUpdated {
            data: Box::new(symbol),
        });
    }

    pub async fn update_latest_prices(&self, prices: Vec<(String, f64, i64)>) {
        if let Err(error) = self.try_update_latest_prices(prices).await {
            tracing::warn!(?error, "paper transition failed after price update");
        }
    }

    pub async fn try_update_latest_prices(
        &self,
        prices: Vec<(String, f64, i64)>,
    ) -> Result<(), PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        self.try_update_latest_prices_locked(prices).await
    }

    async fn try_update_latest_prices_locked(
        &self,
        prices: Vec<(String, f64, i64)>,
    ) -> Result<(), PaperTransitionError> {
        let (updated_symbols, updated_ids, decision_ts_ms) = {
            let mut inner = self.inner.write().await;
            let mut updated_ids = BTreeSet::new();
            let mut decision_ts_ms = Utc::now().timestamp_millis();
            let mut updated_symbols = Vec::new();

            for (inst_id, price, updated_at_ms) in prices {
                if !valid_price(price) {
                    continue;
                }
                if !inner.set_latest_price(&inst_id, price, updated_at_ms) {
                    continue;
                }
                decision_ts_ms = decision_ts_ms.max(updated_at_ms);
                updated_ids.insert(inst_id.clone());
                if let Some(symbol) = inner.symbols.get_mut(&inst_id) {
                    symbol.price = price;
                    symbol.updated_at_ms = updated_at_ms;
                    updated_symbols.push(symbol.clone());
                }
            }
            (updated_symbols, updated_ids, decision_ts_ms)
        };

        for symbol in updated_symbols {
            let _ = self.events.send(BackendEvent::SymbolUpdated {
                data: Box::new(symbol),
            });
        }

        let (active_run_id, run_ids) = {
            let inner = self.inner.read().await;
            (
                inner.strategy_runs.active_run_id().to_string(),
                inner.strategy_runs.ordered_run_ids(),
            )
        };
        for run_id in run_ids {
            let result = self
                .process_updated_prices_for_run(&run_id, &updated_ids, decision_ts_ms)
                .await;
            match result {
                Ok(()) => {}
                Err(error) if run_id == active_run_id => return Err(error),
                Err(error) => {
                    tracing::warn!(%run_id, ?error, "shadow strategy price processing failed")
                }
            }
        }

        let paper = self.paper_snapshot().await;
        if !paper.positions.is_empty() {
            let _ = self.events.send(BackendEvent::PaperUpdated {
                data: Box::new(paper),
            });
        }
        for snapshot in self.strategy_run_snapshots().await {
            if snapshot.mode == StrategyRunMode::ShadowPaper.as_str() {
                let _ = self.events.send(BackendEvent::StrategyRunUpdated {
                    data: Box::new(snapshot),
                });
            }
        }
        Ok(())
    }

    async fn process_updated_prices_for_run(
        &self,
        strategy_run_id: &str,
        updated_ids: &BTreeSet<String>,
        decision_ts_ms: i64,
    ) -> Result<(), PaperTransitionError> {
        let (config, open_ids) = {
            let inner = self.inner.read().await;
            let runtime = inner.strategy_runs.get(strategy_run_id)?;
            (runtime.config(), runtime.paper().open_position_inst_ids())
        };
        let mut closed_ids = BTreeSet::new();
        for inst_id in open_ids {
            if !updated_ids.contains(&inst_id) {
                continue;
            }
            let (decision, prices, position_event) = {
                let inner = self.inner.read().await;
                let prices = inner.price_map();
                let runtime = inner.strategy_runs.get(strategy_run_id)?;
                let paper = runtime.paper().snapshot(&prices);
                let decision = evaluate_auto_exit(&inst_id, &paper, config);
                let position_event = decision.as_ref().and_then(|_| {
                    let position = paper
                        .positions
                        .iter()
                        .find(|position| position.inst_id == inst_id)?;
                    let symbol = inner.symbols.get(&inst_id)?;
                    Some(PositionContextEvent::new(
                        runtime.paper().strategy_identity(),
                        runtime.paper().run_id(),
                        position,
                        symbol,
                        &paper,
                        inner.symbols.get("BTC-USDT-SWAP"),
                        PositionContextEventKind::ExitSignal,
                        decision_ts_ms,
                    ))
                });
                (decision, prices, position_event)
            };
            if let Some(decision) = decision {
                self.queue_observability(strategy_run_id, None, position_event)
                    .await;
                self.apply_strategy_decision_locked(
                    strategy_run_id,
                    decision,
                    &prices,
                    0,
                    decision_ts_ms,
                )
                .await?;
                closed_ids.insert(inst_id);
            }
        }

        let entry_block_reason = self.entry_observation_block_reason().await;
        for inst_id in updated_ids {
            if closed_ids.contains(inst_id) {
                continue;
            }
            let (symbol, mut evaluation, prices, paper, identity, run_id, btc) = {
                let inner = self.inner.read().await;
                let Some(symbol) = inner.symbols.get(inst_id).cloned() else {
                    continue;
                };
                let prices = inner.price_map();
                let runtime = inner.strategy_runs.get(strategy_run_id)?;
                let paper = runtime.paper().snapshot(&prices);
                let evaluation =
                    evaluate_auto_strategy_observed_at(&symbol, &paper, config, decision_ts_ms);
                (
                    symbol,
                    evaluation,
                    prices,
                    paper,
                    runtime.paper().strategy_identity().clone(),
                    runtime.paper().run_id().to_string(),
                    inner.symbols.get("BTC-USDT-SWAP").cloned(),
                )
            };
            if matches!(
                evaluation.decision.as_ref(),
                Some(AutoStrategyDecision::Open { .. })
            ) {
                if let (Some(candidate), Some(reason)) =
                    (evaluation.candidate.as_mut(), entry_block_reason.as_deref())
                {
                    candidate.reject_by_risk(reason);
                }
            }
            let candidate_event = evaluation.candidate.take().map(|candidate| {
                StrategyCandidateEvent::new(
                    &identity,
                    &run_id,
                    &symbol,
                    &paper,
                    btc.as_ref(),
                    candidate,
                    decision_ts_ms,
                )
            });
            let position_event = paper
                .positions
                .iter()
                .find(|position| position.inst_id == symbol.inst_id)
                .map(|position| {
                    PositionContextEvent::new(
                        &identity,
                        &run_id,
                        position,
                        &symbol,
                        &paper,
                        btc.as_ref(),
                        PositionContextEventKind::Checkpoint,
                        decision_ts_ms,
                    )
                });
            self.queue_observability(strategy_run_id, candidate_event, position_event)
                .await;

            let Some(decision) = evaluation.decision else {
                continue;
            };
            if matches!(decision, AutoStrategyDecision::Open { .. }) && entry_block_reason.is_some()
            {
                continue;
            }
            let score = decision_score(&symbol, &decision);
            self.apply_strategy_decision_locked(
                strategy_run_id,
                decision,
                &prices,
                score,
                decision_ts_ms,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn try_update_latest_prices_from_rest(
        &self,
        prices: Vec<(String, f64, i64)>,
    ) -> Result<(), PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        let observed_ids = prices
            .iter()
            .map(|(inst_id, _, _)| inst_id.as_str())
            .collect::<BTreeSet<_>>();
        let positions_match = {
            let inner = self.inner.read().await;
            inner
                .strategy_runs
                .active()
                .paper()
                .open_position_inst_ids()
                .iter()
                .all(|inst_id| observed_ids.contains(inst_id.as_str()))
        };
        {
            let mut inner = self.inner.write().await;
            let sequence = inner.next_risk_event_sequence();
            inner.risk.handle(AccountEventEnvelope {
                event_id: format!("rest-reconcile-{sequence}"),
                stream: "rest:account".to_string(),
                sequence,
                event: AccountEvent::RestReconciled { positions_match },
            });
        }
        self.try_update_latest_prices_locked(prices).await
    }

    pub async fn update_symbol_price_from_websocket(
        &self,
        inst_id: &str,
        price: f64,
        event_ts_ms: i64,
        received_at_ms: i64,
        max_lag_ms: i64,
    ) -> Option<SymbolSnapshot> {
        let _transition = self.account_event_queue.lock().await;
        let event_result = {
            let mut inner = self.inner.write().await;
            inner.risk.handle(AccountEventEnvelope {
                event_id: format!("ws-ticker-{inst_id}-{event_ts_ms}"),
                stream: format!("okx:ticker:{inst_id}"),
                sequence: event_ts_ms.max(0) as u64,
                event: AccountEvent::MarketData {
                    event_ts_ms,
                    received_at_ms,
                    max_lag_ms,
                },
            })
        };
        if matches!(
            event_result,
            AccountEventResult::Duplicate | AccountEventResult::Stale { .. }
        ) {
            return self.inner.read().await.symbols.get(inst_id).cloned();
        }
        if let Err(error) = self
            .try_update_latest_prices_locked(vec![(inst_id.to_string(), price, event_ts_ms)])
            .await
        {
            tracing::warn!(
                ?error,
                "paper transition failed after WebSocket price update"
            );
        }
        self.inner.read().await.symbols.get(inst_id).cloned()
    }

    pub async fn ticker_sync_inst_ids(&self, fixed_watchlist: &[String]) -> Vec<String> {
        let inner = self.inner.read().await;
        let mut ids = BTreeSet::new();
        ids.extend(fixed_watchlist.iter().cloned());
        ids.extend(inner.strategy_runs.open_position_inst_ids());
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
    ) -> Result<PaperAccountSnapshot, PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        self.ensure_entry_allowed().await?;
        let committed_at_ms = Utc::now().timestamp_millis();
        let (active_run_id, mut candidate, prices, price) = {
            let inner = self.inner.read().await;
            (
                inner.strategy_runs.active_run_id().to_string(),
                inner.strategy_runs.active().paper().clone(),
                inner.price_map(),
                inner.price_for(&order.inst_id),
            )
        };
        let Some(price) = price else {
            let error = PaperError::PriceUnavailable(order.inst_id.clone());
            let intent = PersistedOrderIntent::rejected_open(
                &candidate,
                &order,
                0,
                committed_at_ms,
                error.to_string(),
            );
            return Err(self.persist_rejection(intent, error).await);
        };
        let available_balance = candidate.snapshot(&prices).available_balance;
        let trade = match candidate.open(order.clone(), price, available_balance, committed_at_ms) {
            Ok(trade) => trade,
            Err(error) => {
                let intent = PersistedOrderIntent::rejected_open(
                    &candidate,
                    &order,
                    0,
                    committed_at_ms,
                    error.to_string(),
                );
                return Err(self.persist_rejection(intent, error).await);
            }
        };
        let intent = PersistedOrderIntent::accepted_open(&candidate, &order, &trade, 0);
        self.commit_paper_candidate(
            &active_run_id,
            candidate,
            &prices,
            "paper_open",
            Some(intent),
            committed_at_ms,
        )
        .await
    }

    pub async fn close_paper_position(
        &self,
        inst_id: &str,
    ) -> Result<PaperAccountSnapshot, PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        let committed_at_ms = Utc::now().timestamp_millis();
        let (active_run_id, mut candidate, prices, price, prior) = {
            let inner = self.inner.read().await;
            let prices = inner.price_map();
            let prior = inner
                .strategy_runs
                .active()
                .paper()
                .snapshot(&prices)
                .positions
                .into_iter()
                .find(|position| position.inst_id == inst_id);
            let price = inner.price_for(inst_id);
            (
                inner.strategy_runs.active_run_id().to_string(),
                inner.strategy_runs.active().paper().clone(),
                prices,
                price,
                prior,
            )
        };
        let Some(price) = price else {
            let error = PaperError::PriceUnavailable(inst_id.to_string());
            let intent = PersistedOrderIntent::rejected_close(
                &candidate,
                inst_id,
                prior
                    .as_ref()
                    .map(|position| position.side)
                    .unwrap_or(PaperSide::Long),
                0,
                prior
                    .as_ref()
                    .map(|position| position.primary_signal.as_str())
                    .unwrap_or("manual"),
                "manual close",
                Vec::new(),
                committed_at_ms,
                error.to_string(),
            );
            return Err(self.persist_rejection(intent, error).await);
        };
        let trade = match candidate.close(inst_id, price, committed_at_ms) {
            Ok(trade) => trade,
            Err(error) => {
                let side = prior
                    .as_ref()
                    .map(|position| position.side)
                    .unwrap_or(PaperSide::Long);
                let intent = PersistedOrderIntent::rejected_close(
                    &candidate,
                    inst_id,
                    side,
                    0,
                    prior
                        .as_ref()
                        .map(|position| position.primary_signal.as_str())
                        .unwrap_or("manual"),
                    "manual close",
                    Vec::new(),
                    committed_at_ms,
                    error.to_string(),
                );
                return Err(self.persist_rejection(intent, error).await);
            }
        };
        let intent = PersistedOrderIntent::accepted_close(&candidate, &trade, 0);
        self.commit_paper_candidate(
            &active_run_id,
            candidate,
            &prices,
            "paper_close",
            Some(intent),
            committed_at_ms,
        )
        .await
    }

    pub async fn run_auto_strategy_for_symbol(
        &self,
        symbol: &SymbolSnapshot,
        config: AutoStrategyConfig,
    ) -> Result<Option<PaperAccountSnapshot>, PaperTransitionError> {
        self.run_auto_strategy_for_symbol_at(symbol, config, Utc::now().timestamp_millis())
            .await
    }

    pub async fn run_auto_strategy_for_symbol_at(
        &self,
        symbol: &SymbolSnapshot,
        active_config: AutoStrategyConfig,
        now_ms: i64,
    ) -> Result<Option<PaperAccountSnapshot>, PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        let (active_run_id, run_ids) = {
            let mut inner = self.inner.write().await;
            let active = inner.strategy_runs.active();
            if active.config() != active_config {
                return Err(StrategyRuntimeError::ConfigHashMismatch(
                    active.paper().strategy_identity().experiment_key(),
                )
                .into());
            }
            inner.set_latest_price(&symbol.inst_id, symbol.price, symbol.updated_at_ms);
            (
                inner.strategy_runs.active_run_id().to_string(),
                inner.strategy_runs.ordered_run_ids(),
            )
        };
        let mut active_result = None;
        for run_id in run_ids {
            let config = if run_id == active_run_id {
                active_config
            } else {
                self.inner.read().await.strategy_runs.get(&run_id)?.config()
            };
            match self
                .run_auto_strategy_for_run_at(&run_id, symbol, config, now_ms)
                .await
            {
                Ok(result) if run_id == active_run_id => active_result = result,
                Ok(_) => {}
                Err(error) if run_id == active_run_id => return Err(error),
                Err(error) => {
                    tracing::warn!(%run_id, ?error, "shadow strategy evaluation failed")
                }
            }
        }
        Ok(active_result)
    }

    async fn run_auto_strategy_for_run_at(
        &self,
        run_id: &str,
        symbol: &SymbolSnapshot,
        config: AutoStrategyConfig,
        now_ms: i64,
    ) -> Result<Option<PaperAccountSnapshot>, PaperTransitionError> {
        let (mut evaluation, prices, paper, identity, run_id, btc) = {
            let inner = self.inner.read().await;
            let prices = inner.price_map();
            let runtime = inner.strategy_runs.get(run_id)?;
            let paper = runtime.paper().snapshot(&prices);
            (
                evaluate_auto_strategy_observed_at(symbol, &paper, config, now_ms),
                prices,
                paper,
                runtime.paper().strategy_identity().clone(),
                runtime.paper().run_id().to_string(),
                inner.symbols.get("BTC-USDT-SWAP").cloned(),
            )
        };
        let entry_block_reason = if matches!(
            evaluation.decision.as_ref(),
            Some(AutoStrategyDecision::Open { .. })
        ) {
            self.entry_observation_block_reason().await
        } else {
            None
        };
        if let (Some(candidate), Some(reason)) =
            (evaluation.candidate.as_mut(), entry_block_reason.as_deref())
        {
            candidate.reject_by_risk(reason);
        }
        let candidate_event = evaluation.candidate.take().map(|candidate| {
            StrategyCandidateEvent::new(
                &identity,
                &run_id,
                symbol,
                &paper,
                btc.as_ref(),
                candidate,
                now_ms,
            )
        });
        let position_event = paper
            .positions
            .iter()
            .find(|position| position.inst_id == symbol.inst_id)
            .map(|position| {
                let kind = if matches!(
                    evaluation.decision.as_ref(),
                    Some(AutoStrategyDecision::Close { .. })
                ) {
                    PositionContextEventKind::ExitSignal
                } else {
                    PositionContextEventKind::Checkpoint
                };
                PositionContextEvent::new(
                    &identity,
                    &run_id,
                    position,
                    symbol,
                    &paper,
                    btc.as_ref(),
                    kind,
                    now_ms,
                )
            });
        self.queue_observability(&run_id, candidate_event, position_event)
            .await;

        let Some(decision) = evaluation.decision else {
            return Ok(None);
        };
        if matches!(decision, AutoStrategyDecision::Open { .. }) && entry_block_reason.is_some() {
            return Ok(None);
        }
        let score = decision_score(symbol, &decision);
        self.apply_strategy_decision_locked(&run_id, decision, &prices, score, now_ms)
            .await
            .map(Some)
    }

    pub async fn prepare_scan(&self) -> Result<(), PaperTransitionError> {
        let Some(persistence) = &self.persistence else {
            return Ok(());
        };
        let _transition = self.account_event_queue.lock().await;
        if !persistence.postgres_ready().await {
            let error = "PostgreSQL readiness check failed";
            self.pause_persistence(error).await;
            return Err(PaperTransitionError::persistence(error));
        }
        if !self.persistence_is_paused().await {
            return Ok(());
        }

        let expected_runs = {
            let inner = self.inner.read().await;
            inner
                .strategy_runs
                .ordered_run_ids()
                .into_iter()
                .map(|run_id| {
                    let runtime = inner
                        .strategy_runs
                        .get(&run_id)
                        .expect("registered strategy runtime exists");
                    (run_id, runtime.paper().strategy_identity().clone())
                })
                .collect::<Vec<_>>()
        };
        let mut restored_runs = Vec::with_capacity(expected_runs.len());
        for (run_id, identity) in expected_runs {
            let restored = persistence
                .load_paper_state_for_run(&identity, &run_id)
                .await
                .map_err(|error| PaperTransitionError::persistence(&error))?
                .ok_or_else(|| {
                    PaperTransitionError::persistence(format!(
                        "missing strategy checkpoint for {run_id}"
                    ))
                })?;
            let equity_curves = persistence
                .load_equity_curves(&identity, &run_id)
                .await
                .map_err(PaperTransitionError::persistence)?;
            restored_runs.push((run_id, restored, equity_curves));
        }
        let dashboard = {
            let mut inner = self.inner.write().await;
            for (run_id, restored, equity_curves) in restored_runs {
                inner
                    .strategy_runs
                    .get_mut(&run_id)?
                    .replace_persisted_state(restored, equity_curves)?;
            }
            inner.persistence = PersistenceHealthSnapshot::healthy(Utc::now().timestamp_millis());
            inner.dashboard()
        };
        let _ = self.events.send(BackendEvent::Snapshot {
            data: Box::new(dashboard.clone()),
        });
        self.rebuild_cache_after_commit(&dashboard).await;
        Ok(())
    }

    pub async fn mark_scan(&self, ts_ms: i64) -> Result<(), PaperTransitionError> {
        let _transition = self.account_event_queue.lock().await;
        let (run_ids, prices) = {
            let inner = self.inner.read().await;
            (inner.strategy_runs.ordered_run_ids(), inner.price_map())
        };
        for run_id in run_ids {
            let paper = self
                .inner
                .read()
                .await
                .strategy_runs
                .get(&run_id)?
                .paper()
                .clone();
            let mut snapshot = paper.snapshot(&prices);
            if self.persistence.is_some() {
                snapshot.persistence = PersistenceHealthSnapshot::healthy(ts_ms);
            }
            let equity_point = PaperEquityPoint::from_snapshot(ts_ms, &snapshot);
            if let Some(persistence) = &self.persistence {
                let (candidate_events, position_context_events) =
                    self.take_observability_events(&run_id).await?;
                let transition = PersistedTransition {
                    event_type: "scan_checkpoint".to_string(),
                    intent: None,
                    candidate_events,
                    position_context_events,
                    state: paper,
                    snapshot,
                    committed_at_ms: ts_ms,
                };
                if let Err(error) = persistence.persist_transition(&transition).await {
                    self.restore_observability_events(
                        &run_id,
                        transition.candidate_events.clone(),
                        transition.position_context_events.clone(),
                    )
                    .await;
                    self.pause_persistence(error.to_string()).await;
                    return Err(PaperTransitionError::persistence(error));
                }
            }
            append_equity_candles(
                self.inner
                    .write()
                    .await
                    .strategy_runs
                    .get_mut(&run_id)?
                    .equity_curves_mut(),
                equity_point,
            );
        }
        let dashboard = {
            let mut inner = self.inner.write().await;
            inner.last_scan_at_ms = Some(ts_ms);
            if self.persistence.is_some() {
                inner.persistence = PersistenceHealthSnapshot::healthy(ts_ms);
            }
            inner.dashboard()
        };
        for snapshot in self.strategy_run_snapshots().await {
            if snapshot.mode == StrategyRunMode::ShadowPaper.as_str() {
                let _ = self.events.send(BackendEvent::StrategyRunUpdated {
                    data: Box::new(snapshot),
                });
            }
        }
        self.rebuild_cache_after_commit(&dashboard).await;
        Ok(())
    }

    pub async fn set_websocket_connected(&self, connected: bool) {
        let _transition = self.account_event_queue.lock().await;
        let dashboard = {
            let mut inner = self.inner.write().await;
            inner.websocket_connected = connected;
            let sequence = inner.next_risk_event_sequence();
            inner.risk.handle(AccountEventEnvelope {
                event_id: format!("websocket-{connected}-{sequence}"),
                stream: "system:websocket".to_string(),
                sequence,
                event: AccountEvent::WebsocketConnection { connected },
            });
            inner.dashboard()
        };
        let _ = self.events.send(BackendEvent::Snapshot {
            data: Box::new(dashboard),
        });
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BackendEvent> {
        self.events.subscribe()
    }

    async fn apply_strategy_decision_locked(
        &self,
        strategy_run_id: &str,
        decision: AutoStrategyDecision,
        prices: &BTreeMap<String, f64>,
        score: u8,
        now_ms: i64,
    ) -> Result<PaperAccountSnapshot, PaperTransitionError> {
        let mut candidate = self
            .inner
            .read()
            .await
            .strategy_runs
            .get(strategy_run_id)?
            .paper()
            .clone();
        match decision {
            AutoStrategyDecision::Close {
                inst_id,
                reason,
                tags,
                execution_price,
                trigger_price,
                ..
            } => {
                let prior = candidate
                    .snapshot(prices)
                    .positions
                    .into_iter()
                    .find(|position| position.inst_id == inst_id);
                let price = execution_price
                    .filter(|price| valid_price(*price))
                    .or_else(|| {
                        prices
                            .get(&inst_id)
                            .copied()
                            .filter(|price| valid_price(*price))
                    });
                let Some(price) = price else {
                    let error = PaperError::PriceUnavailable(inst_id.clone());
                    let intent = PersistedOrderIntent::rejected_close(
                        &candidate,
                        &inst_id,
                        prior
                            .as_ref()
                            .map(|position| position.side)
                            .unwrap_or(PaperSide::Long),
                        score,
                        prior
                            .as_ref()
                            .map(|position| position.primary_signal.as_str())
                            .unwrap_or("automatic_exit"),
                        &reason,
                        tags.iter().map(|tag| tag.label.clone()).collect(),
                        now_ms,
                        error.to_string(),
                    );
                    return Err(self.persist_rejection(intent, error).await);
                };
                let trade = match candidate.close_with_execution_context(
                    &inst_id,
                    price,
                    now_ms,
                    SCALPING_OPTIMIZATION_SOURCE,
                    &reason,
                    tags.clone(),
                    trigger_price,
                ) {
                    Ok(trade) => trade,
                    Err(error) => {
                        let intent = PersistedOrderIntent::rejected_close(
                            &candidate,
                            &inst_id,
                            prior
                                .as_ref()
                                .map(|position| position.side)
                                .unwrap_or(PaperSide::Long),
                            score,
                            prior
                                .as_ref()
                                .map(|position| position.primary_signal.as_str())
                                .unwrap_or("automatic_exit"),
                            &reason,
                            tags.into_iter().map(|tag| tag.label).collect(),
                            now_ms,
                            error.to_string(),
                        );
                        return Err(self.persist_rejection(intent, error).await);
                    }
                };
                let intent = PersistedOrderIntent::accepted_close(&candidate, &trade, score);
                self.commit_paper_candidate(
                    strategy_run_id,
                    candidate,
                    prices,
                    "automatic_close",
                    Some(intent),
                    now_ms,
                )
                .await
            }
            AutoStrategyDecision::Open {
                order,
                reason,
                tags,
            } => {
                self.ensure_entry_allowed().await?;
                let price = prices
                    .get(&order.inst_id)
                    .copied()
                    .filter(|price| valid_price(*price));
                let Some(price) = price else {
                    let error = PaperError::PriceUnavailable(order.inst_id.clone());
                    let intent = PersistedOrderIntent::rejected_open(
                        &candidate,
                        &order,
                        score,
                        now_ms,
                        error.to_string(),
                    );
                    return Err(self.persist_rejection(intent, error).await);
                };
                let available_balance = candidate.snapshot(prices).available_balance;
                let trade = match candidate.open_with_meta_and_tags(
                    order.clone(),
                    price,
                    available_balance,
                    now_ms,
                    SCALPING_OPTIMIZATION_SOURCE,
                    &reason,
                    tags,
                ) {
                    Ok(trade) => trade,
                    Err(error) => {
                        let intent = PersistedOrderIntent::rejected_open(
                            &candidate,
                            &order,
                            score,
                            now_ms,
                            error.to_string(),
                        );
                        return Err(self.persist_rejection(intent, error).await);
                    }
                };
                let opened_context = {
                    let snapshot = candidate.snapshot(prices);
                    let inner = self.inner.read().await;
                    let position = snapshot
                        .positions
                        .iter()
                        .find(|position| position.inst_id == order.inst_id);
                    let symbol = inner.symbols.get(&order.inst_id);
                    position.zip(symbol).map(|(position, symbol)| {
                        PositionContextEvent::new(
                            candidate.strategy_identity(),
                            candidate.run_id(),
                            position,
                            symbol,
                            &snapshot,
                            inner.symbols.get("BTC-USDT-SWAP"),
                            PositionContextEventKind::Opened,
                            now_ms,
                        )
                    })
                };
                self.queue_observability(strategy_run_id, None, opened_context)
                    .await;
                let intent = PersistedOrderIntent::accepted_open(&candidate, &order, &trade, score);
                self.commit_paper_candidate(
                    strategy_run_id,
                    candidate,
                    prices,
                    "automatic_open",
                    Some(intent),
                    now_ms,
                )
                .await
            }
        }
    }

    async fn commit_paper_candidate(
        &self,
        strategy_run_id: &str,
        candidate: PaperState,
        prices: &BTreeMap<String, f64>,
        event_type: &str,
        intent: Option<PersistedOrderIntent>,
        committed_at_ms: i64,
    ) -> Result<PaperAccountSnapshot, PaperTransitionError> {
        let mut snapshot = candidate.snapshot(prices);
        if self.persistence.is_some() {
            snapshot.persistence = PersistenceHealthSnapshot::healthy(committed_at_ms);
        }
        let equity_point = PaperEquityPoint::from_snapshot(committed_at_ms, &snapshot);
        if let Some(persistence) = &self.persistence {
            let (candidate_events, position_context_events) =
                self.take_observability_events(strategy_run_id).await?;
            let transition = PersistedTransition {
                event_type: event_type.to_string(),
                intent,
                candidate_events,
                position_context_events,
                state: candidate.clone(),
                snapshot: snapshot.clone(),
                committed_at_ms,
            };
            if let Err(error) = persistence.persist_transition(&transition).await {
                self.restore_observability_events(
                    strategy_run_id,
                    transition.candidate_events.clone(),
                    transition.position_context_events.clone(),
                )
                .await;
                self.pause_persistence(error.to_string()).await;
                return Err(PaperTransitionError::persistence(error));
            }
        }

        let (run_snapshot, active_dashboard) = {
            let mut inner = self.inner.write().await;
            let is_active = strategy_run_id == inner.strategy_runs.active_run_id();
            let runtime = inner.strategy_runs.get_mut(strategy_run_id)?;
            *runtime.paper_mut() = candidate;
            append_equity_candles(runtime.equity_curves_mut(), equity_point);
            if self.persistence.is_some() {
                inner.persistence = PersistenceHealthSnapshot::healthy(committed_at_ms);
            }
            let run_snapshot = inner
                .paper_snapshot_for_run(strategy_run_id)
                .ok_or_else(|| StrategyRuntimeError::RunNotFound(strategy_run_id.to_string()))?;
            let dashboard = is_active.then(|| inner.dashboard());
            (run_snapshot, dashboard)
        };
        if let Some(dashboard) = active_dashboard {
            let _ = self.events.send(BackendEvent::PaperUpdated {
                data: Box::new(run_snapshot.clone()),
            });
            self.rebuild_cache_after_commit(&dashboard).await;
        } else {
            let _ = self.events.send(BackendEvent::StrategyRunUpdated {
                data: Box::new(run_snapshot.clone()),
            });
        }
        Ok(run_snapshot)
    }

    async fn persist_rejection(
        &self,
        intent: PersistedOrderIntent,
        paper_error: PaperError,
    ) -> PaperTransitionError {
        if let Some(persistence) = &self.persistence {
            if let Err(error) = persistence.persist_rejection(&intent).await {
                self.pause_persistence(error.to_string()).await;
                return PaperTransitionError::persistence(error);
            }
        }
        PaperTransitionError::Paper(paper_error)
    }

    async fn queue_observability(
        &self,
        strategy_run_id: &str,
        candidate: Option<StrategyCandidateEvent>,
        position: Option<PositionContextEvent>,
    ) {
        if self.persistence.is_none() {
            return;
        }
        let mut inner = self.inner.write().await;
        let Ok(runtime) = inner.strategy_runs.get_mut(strategy_run_id) else {
            tracing::warn!(%strategy_run_id, "dropping observability for missing strategy runtime");
            return;
        };
        if let Some(candidate) = candidate {
            runtime.queue_candidate_event(candidate);
        }
        if let Some(position) = position {
            runtime.queue_position_context_event(position);
        }
    }

    async fn take_observability_events(
        &self,
        strategy_run_id: &str,
    ) -> Result<(Vec<StrategyCandidateEvent>, Vec<PositionContextEvent>), StrategyRuntimeError>
    {
        Ok(self
            .inner
            .write()
            .await
            .strategy_runs
            .get_mut(strategy_run_id)?
            .take_observability_events())
    }

    async fn restore_observability_events(
        &self,
        strategy_run_id: &str,
        candidates: Vec<StrategyCandidateEvent>,
        positions: Vec<PositionContextEvent>,
    ) {
        if let Ok(runtime) = self
            .inner
            .write()
            .await
            .strategy_runs
            .get_mut(strategy_run_id)
        {
            runtime.restore_observability_events(candidates, positions);
        }
    }

    async fn entry_observation_block_reason(&self) -> Option<String> {
        let inner = self.inner.read().await;
        if inner.persistence.status == PersistenceStatus::PersistencePaused {
            return Some("persistence_paused".to_string());
        }
        inner
            .risk
            .entry_block_reason()
            .map(|reason| format!("account_close_only:{reason}"))
    }

    async fn ensure_entry_allowed(&self) -> Result<(), PaperTransitionError> {
        let health = self.persistence_health().await;
        if health.status == PersistenceStatus::PersistencePaused {
            return Err(PaperTransitionError::PersistencePaused(
                health
                    .last_error
                    .unwrap_or_else(|| "PostgreSQL is unavailable".to_string()),
            ));
        }
        if let Some(reason) = self.inner.read().await.risk.entry_block_reason() {
            return Err(PaperTransitionError::RiskCloseOnly(reason));
        }
        Ok(())
    }

    async fn persistence_is_paused(&self) -> bool {
        self.inner.read().await.persistence.status == PersistenceStatus::PersistencePaused
    }

    async fn pause_persistence(&self, error: impl Into<String>) {
        let dashboard = {
            let mut inner = self.inner.write().await;
            let last_committed_at_ms = inner.persistence.last_committed_at_ms;
            inner.persistence = PersistenceHealthSnapshot::paused(error);
            inner.persistence.last_committed_at_ms = last_committed_at_ms;
            inner.dashboard()
        };
        let _ = self.events.send(BackendEvent::Snapshot {
            data: Box::new(dashboard),
        });
    }

    async fn rebuild_cache_after_commit(&self, dashboard: &DashboardSnapshot) {
        let Some(persistence) = &self.persistence else {
            return;
        };
        let result = persistence.rebuild_cache(dashboard).await;
        let available = result.is_ok();
        {
            let mut inner = self.inner.write().await;
            let sequence = inner.next_risk_event_sequence();
            inner.risk.handle(AccountEventEnvelope {
                event_id: format!("redis-health-{available}-{sequence}"),
                stream: "system:redis".to_string(),
                sequence,
                event: AccountEvent::RedisHealth { available },
            });
        }
        if let Err(error) = result {
            tracing::warn!(?error, "Redis cache refresh failed after PostgreSQL commit");
        }
    }
}

fn valid_price(price: f64) -> bool {
    price > 0.0 && price.is_finite()
}

fn decision_score(symbol: &SymbolSnapshot, decision: &AutoStrategyDecision) -> u8 {
    let AutoStrategyDecision::Open { order, .. } = decision else {
        return 0;
    };
    match order.primary_signal.as_deref() {
        Some(signal) if signal.starts_with("trend_") => symbol.trend_score.value,
        Some(signal) if signal.starts_with("range_") => symbol.range_score.value,
        Some(signal) if signal.starts_with("pattern_") => symbol
            .pattern_signals
            .iter()
            .map(|pattern| pattern.trade_score)
            .max()
            .unwrap_or_default(),
        _ => symbol.trend_score.value.max(symbol.range_score.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dashboard_exposes_equity_curves_without_closed_positions() {
        let point = PaperEquityPoint {
            timestamp_ms: 1_000,
            equity: 10_004.0,
            realized_pnl: 0.0,
            unrealized_pnl: 4.0,
            open_positions_count: 1,
        };
        let mut curves = PaperEquityCurves::new();
        append_equity_candles(&mut curves, point);
        let state = RadarState::new(None, PaperState::default(), curves);

        let paper = state.paper_snapshot().await;

        assert_eq!(paper.closed_position_count, 0);
        assert_eq!(paper.equity_curves["1d"].len(), 1);
        assert_eq!(paper.equity_curves["1d"][0].close_equity, 10_004.0);
    }
}
