mod config;
mod database;
mod kafka;

use crate::config::read_database_config;
use crate::config::read_kafka_config;
use crate::kafka::consumer_task::spawn_kafka_consumer_task;
use crate::kafka::kafka_connection::create_kafka_consumer;
use crate::kafka::message_processor::BackupMessageProcessor;
use axum_health::spawn_health_server;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::postgres::init_db;
use std::error::Error;
use std::sync::Arc;
use tokio::signal::unix::{SignalKind, signal};
use tracing::{error, info};

#[tokio::main]
async fn main() {
    register_panic_logger();
    setup_nais_otel().unwrap();
    info!("Starter applikasjon");
    match run_app().await {
        Ok(_) => {
            info!("Applikasjonen avsluttet uten feil");
        }
        Err(e) => {
            error!("Feil ved kjøring av applikasjon, avslutter: {}", e);
            error!("Error details: {:?}", e);
            error!("Error source chain:");
            let mut source = e.source();
            let mut level = 1;
            while let Some(err) = source {
                error!("  Level {}: {}", level, err);
                source = err.source();
                level += 1;
            }
        }
    };
    info!("Main funksjon ferdig, applikasjon avsluttet");
}

async fn run_app() -> Result<(), Box<dyn Error>> {
    let config = config::Config::from_default_file()?;
    info!("Konfigurasjon lastet: {:?}", config);
    let kafka_config = read_kafka_config()?;
    info!("Kafka konfigurasjon lastet: {:?}", kafka_config);

    let app_state = Arc::new(AppState::new());
    let http_server_task = spawn_health_server(app_state.clone());
    info!("HTTP server startet");
    let db_config = read_database_config()?;
    info!("Database config: {:?}", db_config);
    let pg_pool = init_db(db_config).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;
    let hwm_version = *kafka_config.hwm_version;
    let consumer = create_kafka_consumer(
        app_state.clone(),
        pg_pool.clone(),
        kafka_config,
        &config.topics_as_str_slice(),
    )?;
    let kafka_task = spawn_kafka_consumer_task(
        consumer,
        hwm_version,
        pg_pool.clone(),
        BackupMessageProcessor,
    );
    let signal = await_signal();
    app_state.set_has_started(true);
    info!("Alle tjenester startet, applikasjon kjører");
    tokio::select! {
        result = http_server_task => {
            match result {
                Ok(Ok(())) => info!("HTTP server stoppet."),
                Ok(Err(e)) => return Err(e.into()),
                Err(join_error) => return Err(Box::new(join_error)),
            }
        }
        result = kafka_task => {
            match result {
                Ok(Ok(())) => info!("Kafka consumer stoppet."),
                Ok(Err(e)) => return Err(e.into()),
                Err(join_error) => return Err(Box::new(join_error)),
            }
        }
        result = signal => {
            match result {
                Ok(signal) => info!("Signal '{}' mottatt, avslutter....", signal),
                Err(e) => return Err(e),
            }
        }
    }
    app_state.set_is_alive(false);
    let _ = pg_pool.close().await;
    info!("Pg pool lukket");
    Ok(())
}

async fn await_signal() -> Result<String, Box<dyn Error>> {
    let mut term_signal = signal(SignalKind::terminate())?;
    let mut interrupt_signal = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term_signal.recv() => Ok("SIGTERM".to_string()),
        _ = interrupt_signal.recv() => Ok("SIGINT".to_string())
    }
}
