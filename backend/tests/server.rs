use alphapulse_okx_backend::{
    auto_strategy::AutoStrategyConfig, config::AppConfig, paper::PaperAccountSnapshot,
    server::build_router, state::RadarState, strategy_identity::StrategyIdentity,
};
use axum::{
    body::{to_bytes, Body},
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

#[tokio::test]
async fn strategy_run_routes_require_experiment_and_run_identity() {
    let state = RadarState::default();
    let shadow_config = AutoStrategyConfig {
        take_profit_margin_pct: 0.80,
        ..AutoStrategyConfig::default()
    };
    let shadow_run_id = "v0.1.3-session-guard-shadow-1";
    state
        .register_shadow_strategy(
            StrategyIdentity::research_variant_from_config(
                "v0.1.3",
                "session_execution_guard",
                "session-guard-api-test-build",
                &shadow_config,
            ),
            shadow_run_id,
            shadow_config,
        )
        .await
        .unwrap();
    let router = build_router(AppConfig::default(), state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/strategy/runs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let runs: Vec<PaperAccountSnapshot> = serde_json::from_slice(&body).unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0].experiment_key, "v0.1.3/baseline");
    assert_eq!(runs[1].experiment_key, "v0.1.3/session_execution_guard");
    assert_eq!(runs[1].run_id, shadow_run_id);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/strategy/runs/{shadow_run_id}?experiment_key=v0.1.3%2Fsession_execution_guard"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let shadow: PaperAccountSnapshot = serde_json::from_slice(&body).unwrap();
    assert_eq!(shadow.experiment_key, "v0.1.3/session_execution_guard");
    assert_eq!(shadow.run_id, shadow_run_id);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/strategy/runs/{shadow_run_id}?experiment_key=v0.1.3%2Fbaseline"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = router
        .oneshot(
            Request::builder()
                .uri(format!("/api/strategy/runs/{shadow_run_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
