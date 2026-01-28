mod consumer;
mod db;
mod get_env;
mod get_kafka_config;
mod kafka_hwm;

use std::error::Error;
use crate::consumer::create_kafka_consumer;
use crate::db::init_db;
use axum_health::routes;
use health_and_monitoring::simple_app_state::AppState;
use log::LevelFilter;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::json::JsonEncoder;
use log4rs::Config;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::task::JoinHandle;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::error_handling::AppError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn AppError>> {
    setup_nais_otel()?;
    log::info!("Application started");
    let appstate = Arc::new(AppState::new());
    let health_routes = routes(appstate.clone());
    let web_server_task: JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> =
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
            axum::serve(listener, health_routes).await?;
            Ok(())
        });

    let pg_pool = init_db().await.map_err(|err| {
        let error: Box<dyn AppError> = Box::new(DatabaseError {
            message: format!("Failed to initialize database: {}", err),
        });
        error
    })?;
    appstate.set_has_started(true);
    match web_server_task.await {
        Ok(Ok(())) => log::info!("Web server exited successfully."),
        Ok(Err(e)) => log::error!("Web server error: {}", e),
        Err(e) => log::error!("Task join error: {}", e),
    }
    Ok(())
}
