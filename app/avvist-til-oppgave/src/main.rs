mod client;
mod config;
mod db;
mod domain;
mod kafka;
mod message_processor;
mod opprett_oppgave_task;
mod process_hendelselogg_message;
mod process_oppgavehendelse_message;

use crate::config::read_application_config;
use crate::config::read_database_config;
use crate::config::read_kafka_config;
use crate::config::read_oppgave_client_config;
use crate::config::read_token_client_config;
use crate::message_processor::AvvistTilOppgaveMessageProcessor;
use anyhow::Result;
use axum_health::routes;
use client::oppgave_client::OppgaveApiClient;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::error::KafkaError;
use paw_rdkafka_hwm::hwm_message_processor::hwm_process_message;
use paw_rust_base::error::ServerError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::error::DatabaseError;
use paw_sqlx::postgres::{clear_db, init_db};
use std::sync::Arc;
use texas_client::token_client::create_token_client;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    register_panic_logger();
    setup_nais_otel()?;
    tracing::info!("Application started");
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
    let hwm_version = *kafka_config.hwm_version;
    let topics = app_config.topics_as_str();

    let consumer =
        kafka::consumer::create(appstate.clone(), pg_pool.clone(), kafka_config, &topics)
            .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?;

    let reqwest_client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let token_client_config = read_token_client_config()?;
    let token_client = Arc::new(create_token_client(token_client_config, reqwest_client));
    let oppgave_client_config = read_oppgave_client_config()?;
    let oppgave_api_client = Arc::new(OppgaveApiClient::new(oppgave_client_config, token_client));

    let opprett_oppgave_task = opprett_oppgave_task::start_processing_loop(
        pg_pool.clone(),
        oppgave_api_client,
        &app_config,
    );

    let processor = AvvistTilOppgaveMessageProcessor {
        app_config: app_config.clone(),
    };
    let kafka_pg_pool = pg_pool.clone();
    let kafka_consumer_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        loop {
            let msg = consumer.recv().await?.detach();
            hwm_process_message(hwm_version, kafka_pg_pool.clone(), &msg, &processor)
                .await
                .map_err(|e| ServerError::InternalProcessTerminated {
                    process: "KafkaConsumer".to_string(),
                    message: e.to_string(),
                })?;
        }
    });

    appstate.set_has_started(true);

    tokio::select! {
        result = web_server_task => {
            match result {
                Ok(Ok(())) => tracing::info!("Web server exited successfully"),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(ServerError::ThreadSpawn.into()),
            }
        }
        result = kafka_consumer_task => {
            match result {
                Ok(Ok(())) => tracing::info!("Kafka consumer stopped"),
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(ServerError::ThreadSpawn.into()),
            }
        }
        /*result = opprett_oppgave_task => {
            match result {
                Ok(()) => tracing::info!("Opprett oppgave task stopped"),
                Err(e) => return Err(e),
            }
        }*/
    }

    appstate.set_is_alive(false);
    let _ = pg_pool.close().await;
    tracing::info!("PG Pool closed");

    Ok(())
}
