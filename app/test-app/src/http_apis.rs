use crate::app_logic::AppLogic;
use axum::{
    Json,
    extract::{self, State},
    http::StatusCode,
    routing::post,
};
use health::HealthCheck;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tower_http::{
    classify::ServerErrorsAsFailures,
    trace::{Trace, TraceLayer},
};
use tracing::{info, instrument};
use axum::http::HeaderMap;
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::Context;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use log::warn;

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

// Extract trace context from request headers
fn extract_trace_context(headers: &HeaderMap) -> Context {
    let extractor = HeaderExtractor(headers);
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&extractor)
    })
}

pub fn register_http_apis(
    app_state: Arc<dyn HealthCheck + Send + Sync>,
    logic: Arc<AppLogic>,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    tokio::spawn(async move {
        let health_routes = axum_health::routes(app_state);
        let app_routes = api_routes(logic);
        let routes = health_routes.merge(app_routes);
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, routes).await?;
        Ok(())
    })
}

fn api_routes(logic: Arc<AppLogic>) -> axum::Router {
    axum::Router::new()
        .route("/greet", post(greet_handler))
        .route("/greet_json", post(greet_handler_json))
        .layer(TraceLayer::new_for_http())
        .with_state(logic)
}

#[instrument(skip(logic))]
async fn greet_handler(
    State(logic): State<Arc<AppLogic>>,
    body: String,
) -> (StatusCode, [(&'static str, &'static str); 1], String) {
    info!("Processing greet request, type=text/plain");
    let response_body = logic.greet(&body);
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        response_body,
    )
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct GreetRequest {
    name: String,
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct GreetResponse {
    message: String,
}

async fn greet_handler_json(
    State(logic): State<Arc<AppLogic>>,
    headers: HeaderMap,
    extract::Json(payload): extract::Json<GreetRequest>,
) -> Json<GreetResponse> {
    let parent_ctx = extract_trace_context(&headers);
    let span = tracing::info_span!(
        "greet_handler_json",
        otel.kind = "server",
        otel.name = "greet_handler_json"
    );
    let res = span.set_parent(parent_ctx.clone());
    let _guard = span.enter();
    //log warning if res is SetParentError
    match res {
        Ok(_) => {}
        Err(e) => {
            warn!("Failed to set parent context for span: {:?}", e);
        }
    }
    info!("Processing JSON greet request, type=application/json");
    let response_body = logic.greet(&payload.name);
    GreetResponse {
        message: response_body,
    }
    .into()
}
