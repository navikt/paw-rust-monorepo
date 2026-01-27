use crate::app_logic::AppLogic;

use axum::http::HeaderMap;
use axum::{
    extract::{self, State},
    http::StatusCode,
    routing::post,
    Json,
};
use axum_health::paw_tracing::add_otel_trace_layer;
use health_and_monitoring::HealthCheck;
use std::num::NonZeroU16;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{info, instrument};

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
    let router = axum::Router::new()
        .route("/greet", post(greet_handler))
        .route("/greet_json", post(greet_handler_json));
    add_otel_trace_layer(router).with_state(logic)
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
    error: String,
}

impl axum::response::IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let body = serde_json::to_string(&self)
            .unwrap_or_else(|_| "{\"error\":\"Internal Server Error\"}".to_string());
        (
            StatusCode::from_u16(self.code.get().into())
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR),
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
    _headers: HeaderMap,
    extract::Json(payload): extract::Json<GreetRequest>,
) -> Result<Json<GreetResponse>, ErrorResponse> {
    info!("Processing JSON greet request, type=application/json");
    match payload.name.as_str() {
        "NM" => Err(ErrorResponse {
            code: FIVE_HUNDRED,
            error: "Noe gikk galt".to_string(),
        }),
        "MN" => Err(ErrorResponse {
            code: FOUR_HUNDRED,
            error: "Ugyldig data".to_string(),
        }),
        _ => {
            let response_body = logic.greet(&payload.name);
            Ok(GreetResponse {
                message: response_body,
            }
            .into())
        }
    }
}
