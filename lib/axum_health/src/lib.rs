use axum::{extract::State, http::StatusCode, routing::get, Router};
use health::{
    CheckType::{HasStarted, IsAlive, IsReady},
    HealthCheck,
};
use prometheus::{Encoder, TextEncoder};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

pub fn routes(health_check: Arc<dyn HealthCheck + Send + Sync>) -> Router {
    Router::new()
        .route("/internal/isAlive", get(is_alive))
        .route("/internal/isReady", get(is_ready))
        .route("/internal/hasStarted", get(has_started))
        .route("/internal/metrics", get(prometheus))
        .layer(TraceLayer::new_for_http())
        .with_state(health_check)
}

async fn is_alive(
    State(health_check): State<Arc<dyn HealthCheck + Send + Sync>>,
) -> (StatusCode, &'static str) {
    if health_check.check(&IsAlive) != Some(false) {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Service Unavailable")
    }
}

async fn is_ready(
    State(health_check): State<Arc<dyn HealthCheck + Send + Sync>>,
) -> (StatusCode, &'static str) {
    if health_check.check(&IsReady) != Some(false) {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Service Unavailable")
    }
}

async fn has_started(
    State(health_check): State<Arc<dyn HealthCheck + Send + Sync>>,
) -> (StatusCode, &'static str) {
    if health_check.check(&HasStarted) != Some(false) {
        (StatusCode::OK, "ok")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Service Unavailable")
    }
}

async fn prometheus() -> (StatusCode, [(&'static str, &'static str); 1], String) {
    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let metrics = prometheus::gather();
    match encoder.encode(&metrics, &mut buffer) {
        Ok(()) => (
            StatusCode::OK,
            [("Content-Type", "text/plain; version=0.0.4")],
            String::from_utf8(buffer).unwrap(),
        ),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [("Content-Type", "text/plain; version=0.0.4")],
            err.to_string(),
        ),
    }
}
