mod client;
mod config;
mod db;
mod domain;
mod hendelse_processor;
mod kafka;
mod opprett_oppgave_task;

use crate::config::{read_application_config, read_database_config, read_kafka_config};
use anyhow::Result;
use axum_health::routes;
use client::oppgave_client::OppgaveApiClient;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_app_config::config::read_toml_config;
use paw_app_config::read_config_file;
use paw_rdkafka::error::KafkaError;
use paw_rust_base::error::ServerError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::error::DatabaseError;
use paw_sqlx::postgres::init_db;
use std::sync::Arc;
use texas_client::token_client::create_token_client;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    register_panic_logger();
    setup_nais_otel()?;
    log::info!("Application started");
    let appstate = Arc::new(AppState::new());
    let app_config = read_application_config()?;
    let health_routes = routes(appstate.clone());
    let web_server_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, health_routes).await?;
        Ok(())
    });

    let db_config = read_database_config()?;
    let pg_pool = init_db(db_config).await?;

    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .map_err(|e| DatabaseError::MigrateSchema(e))?;

    let kafka_config = read_kafka_config()?;
    let topics = app_config.topics_as_str();
    let hendelselogg_consumer =
        kafka::consumer::create(appstate.clone(), pg_pool.clone(), kafka_config, &topics)
            .map_err(|_| KafkaError::CreateConsumer)?;

    let kafka_processor = hendelse_processor::start_processing_loop(
        hendelselogg_consumer,
        pg_pool.clone(),
        appstate.clone(),
    );

    let reqwest_client = reqwest::Client::new();
    let token_client_config = read_toml_config(read_config_file!("token_client_config.toml"))?;
    let token_client = Arc::new(create_token_client(token_client_config, reqwest_client));
    let oppgave_api_client = Arc::new(OppgaveApiClient::new("todo".to_string(), token_client));
    let opprett_oppgave_task =
        opprett_oppgave_task::start_processing_loop(pg_pool.clone(), oppgave_api_client);

    appstate.set_has_started(true);

    tokio::select! {
        result = web_server_task => {
            match result {
                Ok(Ok(())) => log::info!("Web server exited successfully"),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(ServerError::ThreadSpawn.into()),
            }
        }
        result = kafka_processor => {
            match result {
                Ok(()) => log::info!("Kafka processor stopped"),
                Err(e) => return Err(e),
            }
        }
        result = opprett_oppgave_task => {
            match result {
                Ok(()) => log::info!("Opprett oppgave task stopped"),
                Err(e) => return Err(e),
            }
        }
    }

    appstate.set_is_alive(false);
    let _ = pg_pool.close().await;
    log::info!("PG Pool closed");

    Ok(())
}
