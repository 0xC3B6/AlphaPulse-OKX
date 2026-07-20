use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use alphapulse_okx_backend::{domain::Timeframe, okx::rest::OkxRestClient};
use axum::{extract::Query, http::StatusCode, routing::get, Router};

#[tokio::test]
async fn retries_transient_http_failures() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let route_attempts = attempts.clone();
    let app = Router::new().route(
        "/flaky",
        get(move || {
            let route_attempts = route_attempts.clone();
            async move {
                if route_attempts.fetch_add(1, Ordering::SeqCst) == 0 {
                    (StatusCode::INTERNAL_SERVER_ERROR, "temporary")
                } else {
                    (StatusCode::OK, "ok")
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = OkxRestClient::with_base_url(format!("http://{addr}"));

    let body = client.get_json("/flaky").await.unwrap();

    assert_eq!(body, "ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn backfills_older_history_candles() {
    let app = Router::new()
        .route(
            "/api/v5/market/candles",
            get(|| async {
                r#"{"data":[["3000","3","3","3","3","1","0","0","1"],["2000","2","2","2","2","1","0","0","1"]]}"#
            }),
        )
        .route(
            "/api/v5/market/history-candles",
            get(
                |Query(params): Query<std::collections::HashMap<String, String>>| async move {
                    assert_eq!(params.get("after").map(String::as_str), Some("2000"));
                    r#"{"data":[["1000","1","1","1","1","1","0","0","1"]]}"#
                },
            ),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = OkxRestClient::with_base_url(format!("http://{addr}"));

    let candles = client
        .fetch_candles_with_history("BTC-USDT-SWAP", Timeframe::D1, 3)
        .await
        .unwrap();

    assert_eq!(
        candles
            .iter()
            .map(|candle| candle.ts_ms)
            .collect::<Vec<_>>(),
        vec![1000, 2000, 3000]
    );
}

#[tokio::test]
async fn keeps_recent_candles_when_history_backfill_fails() {
    let app = Router::new()
        .route(
            "/api/v5/market/candles",
            get(|| async {
                r#"{"data":[["3000","3","3","3","3","1","0","0","1"],["2000","2","2","2","2","1","0","0","1"]]}"#
            }),
        )
        .route(
            "/api/v5/market/history-candles",
            get(|| async {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "temporary history failure",
                )
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = OkxRestClient::with_base_url(format!("http://{addr}"));

    let candles = client
        .fetch_candles_with_history("BTC-USDT-SWAP", Timeframe::D1, 3)
        .await
        .unwrap();

    assert_eq!(
        candles
            .iter()
            .map(|candle| candle.ts_ms)
            .collect::<Vec<_>>(),
        vec![2000, 3000]
    );
}
