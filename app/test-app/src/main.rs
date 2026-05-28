mod app_logic;
mod config;
mod consumer;
mod http_apis;

use crate::config::{read_database_config, read_kafka_config};
use crate::consumer::create_consumer;
use crate::http_apis::register_http_apis;
use anyhow::Result;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_rust_base::error::ServerError;
use paw_sqlx::error::DatabaseError;
use paw_sqlx::postgres::init_db;
use rdkafka::consumer::StreamConsumer;
use rdkafka::Message;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::info;
use paw_rdkafka_hwm::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;

const HENDELSELOGG_TOPIC: &str = "paw.arbeidssoker-hendelseslogg-v1";

#[tokio::main]
async fn main() -> Result<()> {
    setup_nais_otel()?;
    info!("Starter test app");

    let app_state = Arc::new(AppState::new());

    let db_config = read_database_config()?;
    let pg_pool = init_db(db_config).await?;
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .map_err(DatabaseError::MigrateSchema)?;
    info!("Database migrert");

    let kafka_config = read_kafka_config()?;
    let hwm_version = *kafka_config.hwm_version;
    let topics = [HENDELSELOGG_TOPIC];

    let consumer =
        create_consumer(app_state.clone(), pg_pool.clone(), kafka_config, &topics)?;
    info!("Kafka consumer opprettet, lytter på {:?}", topics);

    let app_logic = Arc::new(app_logic::AppLogic::new(Arc::from("Hello")));
    let http_server_task = register_http_apis(app_state.clone(), app_logic.clone());

    let consumer_task = spawn_consumer_task(consumer, hwm_version, pg_pool.clone());

    app_state.set_has_started(true);

    let result: Result<()> = tokio::select! {
        result = http_server_task => match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => Err(ServerError::Start.into()),
            Err(_) => Err(ServerError::Start.into()),
        },
        result = consumer_task => match result {
            Ok(Ok(())) => { info!("Consumer stoppet"); Ok(()) }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ServerError::ThreadSpawn.into()),
        },
    };

    app_state.set_is_alive(false);
    let _ = pg_pool.close().await;
    info!("Shutdown complete");

    result
}

fn spawn_consumer_task(
    consumer: StreamConsumer<HwmRebalanceHandler>,
    hwm_version: i16,
    pg_pool: PgPool,
) -> tokio::task::JoinHandle<Result<()>> {
    tokio::spawn(async move {
        use paw_rdkafka_hwm::hwm_functions::update_hwm;
        loop {
            let msg = consumer.recv().await?.detach();
            let topic = msg.topic().to_string();
            let partition = msg.partition();
            let offset = msg.offset();

            let mut tx = pg_pool.begin().await?;
            let hwm_ok = update_hwm(&mut tx, hwm_version, &topic, partition, offset).await?;

            if hwm_ok {
                tracing::debug!(
                    "Melding over HWM: topic={}, partition={}, offset={}",
                    topic, partition, offset
                );
                tx.commit().await?;
            } else {
                tracing::debug!(
                    "Under HWM, ignorerer: topic={}, partition={}, offset={}",
                    topic, partition, offset
                );
            }
        }
    })
}

