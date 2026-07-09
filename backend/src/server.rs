use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    config::AppConfig,
    domain::{ChartSnapshot, Timeframe},
    indicators::fvg::detect_fvgs,
    macro_cycle,
    okx::rest::OkxRestClient,
    paper::{PaperError, PaperOrderRequest},
    runtime,
    state::{BackendEvent, RadarState},
    strategy::{StrategyError, StrategyVersionSnapshot},
    valuation::CoinglassValuationClient,
};

#[derive(Clone)]
struct AppCtx {
    state: RadarState,
    rest: OkxRestClient,
    valuation: CoinglassValuationClient,
    macro_cache: Arc<Mutex<macro_cycle::BtcMacroSnapshotCache>>,
}

pub fn build_router(config: AppConfig, state: RadarState) -> Router {
    let ctx = AppCtx {
        state,
        rest: OkxRestClient::new(),
        valuation: CoinglassValuationClient::new(config.coinglass_api_key.clone()),
        macro_cache: Arc::new(Mutex::new(macro_cycle::BtcMacroSnapshotCache::new(
            Duration::from_secs(60),
        ))),
    };
    Router::new()
        .route("/api/health", get(health))
        .route("/api/snapshot", get(snapshot))
        .route("/api/macro/btc", get(btc_macro))
        .route("/api/symbols/:inst_id/chart", get(symbol_chart))
        .route("/api/paper", get(paper))
        .route("/api/paper/orders", post(open_paper_order))
        .route(
            "/api/paper/positions/:inst_id/close",
            post(close_paper_position),
        )
        .route("/api/strategy/versions", get(strategy_versions))
        .route(
            "/api/strategy/versions/:version_code",
            get(strategy_version),
        )
        .route(
            "/api/strategy/versions/:version_code/start",
            post(start_strategy_version),
        )
        .route(
            "/api/strategy/versions/:version_code/stop",
            post(stop_strategy_version),
        )
        .route(
            "/api/strategy/versions/:version_code/reset",
            post(reset_strategy_version),
        )
        .route(
            "/api/strategy/versions/:version_code/overview",
            get(strategy_version_overview),
        )
        .route(
            "/api/strategy/versions/:version_code/equity",
            get(strategy_version_equity),
        )
        .route(
            "/api/strategy/versions/:version_code/positions",
            get(strategy_version_positions),
        )
        .route(
            "/api/strategy/versions/:version_code/trades",
            get(strategy_version_trades),
        )
        .route(
            "/api/strategy/versions/:version_code/attribution/signals",
            get(strategy_version_signal_attribution),
        )
        .route(
            "/api/strategy/versions/:version_code/attribution/tags",
            get(strategy_version_tag_attribution),
        )
        .route(
            "/api/strategy/versions/:version_code/attribution/combos",
            get(strategy_version_combo_attribution),
        )
        .route(
            "/api/strategy/versions/:version_code/attribution/symbols",
            get(strategy_version_symbol_attribution),
        )
        .route(
            "/api/strategy/versions/:version_code/risk-guard/events",
            get(strategy_version_risk_events),
        )
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(ctx)
}

pub async fn serve(config: AppConfig) -> anyhow::Result<()> {
    let state = RadarState::default();
    let scanner_config = config.clone();
    let scanner_state = state.clone();
    tokio::spawn(async move {
        if let Err(error) = runtime::run_scanner(scanner_config, scanner_state).await {
            tracing::error!(?error, "scanner task exited");
        }
    });

    let app = build_router(config.clone(), state);
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("backend listening on http://{}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn snapshot(State(ctx): State<AppCtx>) -> impl IntoResponse {
    Json(ctx.state.snapshot().await)
}

async fn btc_macro(State(ctx): State<AppCtx>) -> Result<impl IntoResponse, ApiError> {
    let now_ms = Utc::now().timestamp_millis();
    if let Some(snapshot) = ctx.macro_cache.lock().await.get(now_ms) {
        return Ok(Json(snapshot));
    }

    let snapshot = macro_cycle::fetch_btc_macro_snapshot(&ctx.rest, &ctx.valuation)
        .await
        .map_err(ApiError::bad_gateway)?;
    ctx.macro_cache.lock().await.store(now_ms, snapshot.clone());
    Ok(Json(snapshot))
}

async fn symbol_chart(
    State(ctx): State<AppCtx>,
    Path(inst_id): Path<String>,
    Query(query): Query<ChartQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let timeframe = query.timeframe.unwrap_or(Timeframe::M15);
    let limit = query.limit.unwrap_or(180).clamp(30, 300);
    let candles = ctx
        .rest
        .fetch_candles(&inst_id, timeframe, limit)
        .await
        .map_err(ApiError::bad_gateway)?;
    let current_price = candles
        .last()
        .map(|candle| candle.close)
        .ok_or_else(|| ApiError::not_found(format!("no candles for {inst_id}")))?;
    let mut fvgs = detect_fvgs(&candles, timeframe, 0.003, current_price);
    if query.filled == Some(false) {
        fvgs.retain(|zone| !zone.filled);
    }
    fvgs.sort_by(|left, right| {
        left.distance_pct
            .partial_cmp(&right.distance_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let updated_at_ms = candles
        .last()
        .map(|candle| candle.ts_ms)
        .unwrap_or_default();

    Ok(Json(ChartSnapshot {
        inst_id,
        timeframe,
        candles,
        fvgs,
        updated_at_ms,
    }))
}

async fn paper(State(ctx): State<AppCtx>) -> impl IntoResponse {
    Json(ctx.state.paper_snapshot().await)
}

async fn open_paper_order(
    State(ctx): State<AppCtx>,
    Json(order): Json<PaperOrderRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let paper = ctx.state.open_paper_order(order).await?;
    Ok(Json(paper))
}

async fn close_paper_position(
    State(ctx): State<AppCtx>,
    Path(inst_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let paper = ctx.state.close_paper_position(&inst_id).await?;
    Ok(Json(paper))
}

async fn strategy_versions(State(ctx): State<AppCtx>) -> impl IntoResponse {
    Json(ctx.state.strategy_center_snapshot().await)
}

async fn strategy_version(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        ctx.state.strategy_version_snapshot(&version_code).await?,
    ))
}

async fn start_strategy_version(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(ctx.state.start_strategy_version(&version_code).await?))
}

async fn stop_strategy_version(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(ctx.state.stop_strategy_version(&version_code).await?))
}

async fn reset_strategy_version(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(ctx.state.reset_strategy_version(&version_code).await?))
}

async fn strategy_version_overview(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let version = ctx.state.strategy_version_snapshot(&version_code).await?;
    Ok(Json(version.overview))
}

async fn strategy_version_equity(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let version = ctx.state.strategy_version_snapshot(&version_code).await?;
    Ok(Json(version.equity))
}

async fn strategy_version_positions(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let version = ctx.state.strategy_version_snapshot(&version_code).await?;
    Ok(Json(version.paper.positions))
}

async fn strategy_version_trades(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
    Query(query): Query<TradesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let version = ctx.state.strategy_version_snapshot(&version_code).await?;
    Ok(Json(filter_trades(version, query)))
}

async fn strategy_version_signal_attribution(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        ctx.state
            .strategy_version_snapshot(&version_code)
            .await?
            .signal_attribution,
    ))
}

async fn strategy_version_tag_attribution(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        ctx.state
            .strategy_version_snapshot(&version_code)
            .await?
            .tag_attribution,
    ))
}

async fn strategy_version_combo_attribution(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        ctx.state
            .strategy_version_snapshot(&version_code)
            .await?
            .combo_attribution,
    ))
}

async fn strategy_version_symbol_attribution(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        ctx.state
            .strategy_version_snapshot(&version_code)
            .await?
            .symbol_attribution,
    ))
}

async fn strategy_version_risk_events(
    State(ctx): State<AppCtx>,
    Path(version_code): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        ctx.state
            .strategy_version_snapshot(&version_code)
            .await?
            .risk_guard_events,
    ))
}

async fn ws_handler(State(ctx): State<AppCtx>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, ctx.state))
}

async fn ws_loop(mut socket: WebSocket, state: RadarState) {
    let snapshot = state.snapshot().await;
    let _ = socket
        .send(Message::Text(
            serde_json::to_string(&BackendEvent::Snapshot { data: snapshot }).unwrap(),
        ))
        .await;

    let mut rx = state.subscribe();
    while let Ok(event) = rx.recv().await {
        if socket
            .send(Message::Text(serde_json::to_string(&event).unwrap()))
            .await
            .is_err()
        {
            break;
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ChartQuery {
    timeframe: Option<Timeframe>,
    limit: Option<usize>,
    filled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TradesQuery {
    symbol: Option<String>,
    side: Option<String>,
    primary_signal: Option<String>,
    tag: Option<String>,
    exit_reason: Option<String>,
    start_time: Option<i64>,
    end_time: Option<i64>,
}

fn filter_trades(
    version: StrategyVersionSnapshot,
    query: TradesQuery,
) -> Vec<crate::paper::PaperClosedPositionSnapshot> {
    version
        .paper
        .position_history
        .into_iter()
        .filter(|trade| {
            query
                .symbol
                .as_ref()
                .is_none_or(|symbol| trade.inst_id.eq_ignore_ascii_case(symbol))
        })
        .filter(|trade| {
            query.side.as_ref().is_none_or(|side| {
                matches!(
                    (side.as_str(), trade.side),
                    ("long", crate::paper::PaperSide::Long)
                        | ("short", crate::paper::PaperSide::Short)
                )
            })
        })
        .filter(|trade| {
            query
                .primary_signal
                .as_ref()
                .is_none_or(|signal| trade.primary_signal == *signal)
        })
        .filter(|trade| {
            query
                .tag
                .as_ref()
                .is_none_or(|tag| trade.tags.iter().any(|trade_tag| trade_tag == tag))
        })
        .filter(|trade| {
            query
                .exit_reason
                .as_ref()
                .is_none_or(|reason| trade.close_reason.contains(reason))
        })
        .filter(|trade| {
            query
                .start_time
                .is_none_or(|start| trade.closed_at_ms >= start)
        })
        .filter(|trade| query.end_time.is_none_or(|end| trade.closed_at_ms <= end))
        .collect()
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn not_found(message: String) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message,
        }
    }

    fn bad_gateway(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: error.to_string(),
        }
    }
}

impl From<PaperError> for ApiError {
    fn from(error: PaperError) -> Self {
        let status = match error {
            PaperError::PriceUnavailable(_) | PaperError::PositionNotFound(_) => {
                StatusCode::NOT_FOUND
            }
            PaperError::EmptyInstrument
            | PaperError::InvalidMargin
            | PaperError::InvalidLeverage
            | PaperError::InsufficientBalance
            | PaperError::OppositePosition => StatusCode::BAD_REQUEST,
        };

        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl From<StrategyError> for ApiError {
    fn from(error: StrategyError) -> Self {
        match error {
            StrategyError::UnknownVersion(message) => Self::not_found(message),
            StrategyError::Paper(error) => error.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status,
            Json(ErrorResponse {
                message: self.message,
            }),
        )
            .into_response()
    }
}
