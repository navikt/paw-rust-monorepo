use std::num::NonZeroU16;
use crate::app_logic::AppLogic;
use axum::{
    Json,
    extract::{self, State},
    http::StatusCode,
    routing::post,
};
use health::HealthCheck;
use std::sync::Arc;
use axum::extract::MatchedPath;
use tokio::task::JoinHandle;
use tower_http::{
    classify::ServerErrorsAsFailures,
    trace::{Trace, TraceLayer},
};
use tracing::{info, warn, instrument, Span, Level, info_span};
use axum::http::{HeaderMap, Request};
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::{Status, TraceContextExt};
use opentelemetry::Context;
use tower_http::trace::OnFailure;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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
        .layer(TraceLayer::new_for_http()
            .make_span_with(|request: &Request<_>| {
                // Log the matched route's path (with placeholders not filled in).
                // Use request.uri() or OriginalUri if you want the real path.
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);
                let parent_ctx = extract_trace_context(request.headers());

                let span = info_span!(
                        "http_request",
                        http.request.method = ?request.method(),
                        http.route = matched_path,
                        some_other_field = tracing::field::Empty,
                    );
                let _res = span.set_parent(parent_ctx);
                span
            }).on_response(|response: &axum::response::Response, _latency: std::time::Duration, span: &Span| {
                let code = response.status().as_u16();
                if code >= 500 {
                    span.set_status(Status::Error {description: "Internal Server Error".into()});
                }
                span.record("http.response.status_code", response.status().as_u16());
        }).on_failure(|_error, _latency, span: &Span| {
            span.set_status(Status::Error {description: "Internal Server Error".into()});
        }))
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

#[derive(serde::Serialize)]
struct ErrorResponse {
    code: NonZeroU16,
    error: String
}

impl axum::response::IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::to_string(&self)
            .unwrap_or_else(|_| "{\"error\":\"Internal Server Error\"}".to_string());
        (
            StatusCode::from_u16(self.code.get().into()).unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR),
            [("Content-Type", "application/json; charset=utf-8")],
            body,
        )
            .into_response()
    }

}

const FIVE_HUNDRED: NonZeroU16 = NonZeroU16::new(500).expect("500 is not zero");
const FOUR_HUNDRED: NonZeroU16 = NonZeroU16::new(400).expect("400 is not zero");

async fn greet_handler_json(
    State(logic): State<Arc<AppLogic>>,
    headers: HeaderMap,
    extract::Json(payload): extract::Json<GreetRequest>,
) -> Result<Json<GreetResponse>, ErrorResponse> {
    info!("Processing JSON greet request, type=application/json");
     match payload.name.as_str() {
        "NM" => {
            Err(ErrorResponse {
                code: FIVE_HUNDRED,
                error: "Noe gikk galt".to_string(),
            })
        },
         "MN" => {
            Err(ErrorResponse {
                code: FOUR_HUNDRED,
                error: "Ugyldig data".to_string(),
            })
         },
        _ => {
            let response_body = logic.greet(&payload.name);
            Ok(GreetResponse {
                message: response_body,
            }
                .into())
        }
    }
}
