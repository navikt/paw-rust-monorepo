mod app_logic;
mod config;
mod consumer;
mod http_apis;

use crate::config::{read_database_config, read_kafka_config};
use crate::http_apis::register_http_apis;
use anyhow::Result;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_rust_base::error::ServerError;
use paw_sqlx::error::DatabaseError;
use paw_sqlx::postgres::{clear_db, init_db};
use std::sync::Arc;
use tracing::info;

const HENDELSELOGG_TOPIC: &str = "paw.arbeidssoker-hendelseslogg-v1";

#[tokio::main]
async fn main() -> Result<()> {
    setup_nais_otel()?;
    info!("Starter test app");

    let app_state = Arc::new(AppState::new());

    let db_config = read_database_config()?;
    let pg_pool = init_db(db_config).await?;
    clear_db(&pg_pool).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await.map_err(DatabaseError::MigrateSchema)?;
    info!("Database migrert");

    let kafka_config = read_kafka_config()?;
    let _hwm_version = *kafka_config.hwm_version;
    let _topics = [HENDELSELOGG_TOPIC];

    let app_logic = Arc::new(app_logic::AppLogic::new(Arc::from("Hello")));
    let http_server_task = register_http_apis(app_state.clone(), app_logic.clone());
    app_state.set_has_started(true);

    let result: Result<()> = tokio::select! {
        result = http_server_task => match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(ServerError::Start.into()),
            Err(_) => Err(ServerError::Start.into()),
        },
    };

    app_state.set_is_alive(false);
    let _ = pg_pool.close().await;
    info!("Shutdown complete");

    result
}
