mod client;
mod config;
mod db;
mod domain;
mod hendelselogg;
mod kafka;
mod message_processor;
mod metrics;
mod opprett_ekstern_oppgave_task;
mod process_oppgavehendelse_message;

#[cfg(test)]
mod test_utils;

use crate::config::read_application_config;
use crate::config::read_database_config;
use crate::config::read_kafka_config;
use crate::config::read_oppgave_client_config;
use crate::config::read_token_client_config;
use crate::kafka::consumer_task::spawn_kafka_consumer_task;
use crate::message_processor::VeilederOppgaveMessageProcessor;
use crate::metrics::metrics_task::spawn_metrics_task;
use crate::opprett_ekstern_oppgave_task::spawn_ekstern_oppgave_task;
use anyhow::Result;
use axum_health::spawn_health_server;
use client::oppgave_client::OppgaveApiClient;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::error::KafkaError;
use paw_rust_base::error::ServerError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::error::DatabaseError;
use paw_sqlx::postgres::init_db;
use std::sync::Arc;
use std::time::Duration;
use metrics::ekstern_oppgave_opprettelse_feil::init_ekstern_oppgave_opprettelse_feil_counter;
use texas_client::token_client::create_token_client;

#[tokio::main]
async fn main() -> Result<()> {
    register_panic_logger();
    setup_nais_otel()?;
    init_ekstern_oppgave_opprettelse_feil_counter();
    tracing::info!("Application started");
    let appstate = Arc::new(AppState::new());
    let app_config = read_application_config()?;

    let db_config = read_database_config()?;
    let pg_pool = init_db(db_config).await?;
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .map_err(DatabaseError::MigrateSchema)?;

    let kafka_config = read_kafka_config()?;
    let hwm_version = *kafka_config.hwm_version;
    let topics = app_config.topics_as_str();

    let consumer =
        kafka::consumer::create(appstate.clone(), pg_pool.clone(), kafka_config, &topics)
            .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?;

    let reqwest_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let token_client_config = read_token_client_config()?;
    let token_client = Arc::new(create_token_client(token_client_config, reqwest_client));
    let oppgave_client_config = read_oppgave_client_config()?;
    let oppgave_api_client = Arc::new(OppgaveApiClient::new(oppgave_client_config, token_client));

    let opprett_ekstern_oppgave_task =
        spawn_ekstern_oppgave_task(pg_pool.clone(), oppgave_api_client, app_config.clone());

    let kafka_consumer_task = spawn_kafka_consumer_task(
        consumer,
        hwm_version,
        pg_pool.clone(),
        VeilederOppgaveMessageProcessor {
            app_config: app_config.clone(),
        },
    );
    let web_server_task = spawn_health_server(appstate.clone());
    let metrikk_task = spawn_metrics_task(pg_pool.clone());

    appstate.set_has_started(true);

    let result: Result<()> = tokio::select! {
        result = web_server_task => match result {
            Ok(Ok(())) => { tracing::info!("Web server exited successfully"); Ok(()) }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ServerError::ThreadSpawn.into()),
        },
        result = kafka_consumer_task => match result {
            Ok(Ok(())) => { tracing::info!("Kafka consumer stopped"); Ok(()) }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ServerError::ThreadSpawn.into()),
        },
        result = opprett_ekstern_oppgave_task => match result {
            Ok(Ok(())) => { tracing::info!("Opprett oppgave task stopped"); Ok(()) }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ServerError::ThreadSpawn.into()),
        },
        result = metrikk_task => match result {
            Ok(()) => { tracing::warn!("Metrikk task stoppet uventet"); Ok(()) }
            Err(join_error) => { tracing::warn!(error = %join_error, "Metrikk task panicked"); Ok(()) }
        },
    };

    appstate.set_is_alive(false);
    let _ = pg_pool.close().await;
    tracing::info!("PG Pool closed og isAlive satt til false");

    result
}
