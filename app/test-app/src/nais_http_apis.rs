use std::sync::Arc;

use axum_health::routes;
use health::HealthCheck;
use tokio::task::JoinHandle;

pub fn register_nais_http_apis(
    app_state: Arc<dyn HealthCheck + Send + Sync>,
) -> JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
    tokio::spawn(async move {
        let routes = routes(app_state);
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, routes).await?;
        Ok(())
    })
}
