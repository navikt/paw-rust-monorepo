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
        .with_state(logic)
}

async fn greet_handler(
    State(logic): State<Arc<AppLogic>>,
    body: String,
) -> (StatusCode, [(&'static str, &'static str); 1], String) {
    let response_body = logic.greet(&body);
    (
        StatusCode::OK,
        [("Content-Type", "text/plain; charset=utf-8")],
        response_body,
    )
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GreetRequest {
    name: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GreetResponse {
    message: String,
}

async fn greet_handler_json(
    State(logic): State<Arc<AppLogic>>,
    extract::Json(payload): extract::Json<GreetRequest>,
) -> Json<GreetResponse> {
    let response_body = logic.greet(&payload.name);
    GreetResponse {
        message: response_body,
    }
    .into()
}
