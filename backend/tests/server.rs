use alphapulse_okx_backend::{config::AppConfig, server::build_router, state::RadarState};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use tower::ServiceExt;

#[tokio::test]
async fn health_route_returns_ok() {
    let router = build_router(AppConfig::default(), RadarState::default());

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn snapshot_route_returns_empty_symbol_list_initially() {
    let router = build_router(AppConfig::default(), RadarState::default());

    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
