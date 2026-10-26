mod database;
mod meldinger;
mod message_processor;
mod regler;
mod reset_db;
mod schema_registry;
mod signing_required_tools;
mod topic_catalog;
mod topic_ids;
mod trace_operation_type;
use std::sync::Arc;
use std::time::Duration;

use message_processor::InternkontrollMessageProcessor;
use paw_kafka::hwm::kafka_connection::create_kafka_consumer_with_sender;
use paw_kafka::hwm::stream::stream_config::PawKafkaStreamConfig;
use paw_kafka::hwm::{
    hwm_message_processor::{ProcessorError, hwm_process_message},
    rebalance::topic_partition_update::TopicPartitionUpdate,
    stream::{
        paw_kafka_stream::PawKafkaStream,
        stream_wrapper::{PawKafkaConsumerStream, init_stream_wrapper_metrics},
    },
};
use paw_kafka::kafka_config::KafkaConfig;
use paw_kafka::signing::RecordVerifier;
use paw_kafka::topics::get_topic_names;
use paw_observability::server::spawn_health_server;
use paw_observability::{health::simple_app_state, otel::setup_nais_otel};
use paw_rust_base::{
    await_signal::await_signal, env::runtime_env, panic_logger::register_panic_logger,
};
use paw_rust_base::{config::read_toml_config, read_config_file};
use paw_sqlx::config::DatabaseConfig;
use std::error::Error;
use tokio::sync::mpsc;
use tokio::time::interval;
use tracing::info;

use crate::meldinger::Dekodere;
use crate::regler::hele_trace_er_signert::regel_hele_trace_er_signert;
use crate::regler::regler_task::spawn_regel_task;
use crate::reset_db::reset_db_if_needed;
use crate::schema_registry::create_schema_registry_settings;
use crate::signing_required_tools::SigningRequiredFrom;
use crate::topic_catalog::{subscribed_topics, topic_priorities};

const REGEL_BATCH_SIZE: std::num::NonZeroU32 = std::num::NonZeroU32::new(100).unwrap();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    register_panic_logger();
    setup_nais_otel().unwrap();
    init_stream_wrapper_metrics();
    let record_verifier = RecordVerifier::from_embedded_keys()?;
    run_app(record_verifier).await
}

async fn run_app(record_verifier: RecordVerifier) -> Result<(), Box<dyn Error>> {
    let kafka_config: KafkaConfig = read_toml_config(read_config_file!("kafka_config.toml"))?;
    let database_config: DatabaseConfig =
        read_toml_config(read_config_file!("database_config.toml"))?;
    let app_state = Arc::new(simple_app_state::AppState::new());
    let http_server_task = spawn_health_server(app_state.clone());
    let hwm_version = *kafka_config.hwm_version;
    let runtime_env = runtime_env();
    let subscribed_topics = subscribed_topics();
    let topics = get_topic_names(&runtime_env, &subscribed_topics);
    let pg_pool = paw_sqlx::postgres::init_db(database_config).await?;
    reset_db_if_needed(&pg_pool, hwm_version).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;
    let mut conn = pg_pool.acquire().await?;
    database::topic::register_topics(&mut conn, &topics).await?;
    let topic_ids = database::topic::load_topic_ids(&mut conn).await?;
    drop(conn);
    let (topic_update_sender, topic_update_receiver) =
        mpsc::unbounded_channel::<TopicPartitionUpdate>();
    let signing_required_from = SigningRequiredFrom::for_runtime(&runtime_env);
    let dekodere = Dekodere::new(
        topics.iter().copied().zip(subscribed_topics.iter()),
        create_schema_registry_settings(&runtime_env)?,
    );
    let topic_priorities = topic_priorities(&runtime_env);
    let stream_config = PawKafkaStreamConfig::from_kafka_config(&kafka_config, topic_priorities)?;
    let consumer = create_kafka_consumer_with_sender(
        app_state.clone(),
        pg_pool.clone(),
        kafka_config,
        &topics,
        topic_update_sender,
    )?;
    let stream = PawKafkaConsumerStream::new(
        topic_update_receiver,
        consumer,
        pg_pool.clone(),
        stream_config,
    );
    let regler = vec![regel_hele_trace_er_signert()];
    let regel_task = spawn_regel_task(
        pg_pool.clone(),
        app_state.clone(),
        interval(Duration::from_mins(5)),
        regler,
        REGEL_BATCH_SIZE,
    );
    let kafka_task = tokio::spawn({
        let state = app_state.clone();
        let pg_pool = pg_pool.clone();
        async move {
            let message_processor = InternkontrollMessageProcessor::new(
                signing_required_from,
                record_verifier,
                topic_ids,
                dekodere,
            );
            let mut paw_stream = stream;
            while state.is_alive() {
                let (next_stream, msg) = paw_stream.receive().await?;
                paw_stream = next_stream;
                if let Some(msg) = msg {
                    hwm_process_message(hwm_version, pg_pool.clone(), &msg, &message_processor)
                        .await?
                }
            }
            Ok::<(), ProcessorError>(())
        }
    });
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
                Ok(Err(e)) => return Err(e),
                Err(join_error) => return Err(Box::new(join_error)),
            }
        }
        result = signal => {
            match result {
                Ok(signal) => info!("Signal '{}' mottatt, avslutter....", signal),
                Err(e) => return Err(e),
            }
        }
        result = regel_task => {
            match result {
                Ok(Ok(())) => info!("Regel task stoppet."),
                Ok(Err(e)) => return Err(e.into()),
                Err(join_error) => return Err(Box::new(join_error)),
            }
        }
    }
    app_state.set_is_alive(false);
    let _ = pg_pool.close().await;
    info!("Pg pool lukket");
    Ok(())
}
