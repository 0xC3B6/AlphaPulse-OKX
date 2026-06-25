use std::net::SocketAddr;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    config::AppConfig,
    runtime,
    state::{BackendEvent, RadarState},
};

#[derive(Clone)]
struct AppCtx {
    state: RadarState,
}

pub fn build_router(_config: AppConfig, state: RadarState) -> Router {
    let ctx = AppCtx { state };
    Router::new()
        .route("/api/health", get(health))
        .route("/api/snapshot", get(snapshot))
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
