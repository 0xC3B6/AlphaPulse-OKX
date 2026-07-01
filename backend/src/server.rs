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
