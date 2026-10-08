mod message_processor;

use std::sync::Arc;

use axum_health::spawn_health_server;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use message_processor::InternkontrollMessageProcessor;
use paw_app_config::{config::read_toml_config, read_config_file};
use paw_kafka::kafka_config::KafkaConfig;
use paw_kafka::hwm::kafka_connection::create_kafka_consumer_with_sender;
use paw_kafka::hwm::stream::stream_config::PawKafkaStreamConfig;
use paw_kafka::hwm::stream::topic_priority::TopicPriorityList;
use paw_kafka::hwm::{
    hwm_message_processor::{ProcessorError, hwm_process_message},
    rebalance::topic_partition_update::TopicPartitionUpdate,
    stream::{
        paw_kafka_stream::PawKafkaStream,
        stream_wrapper::{PawKafkaConsumerStream, init_stream_wrapper_metrics},
    },
};
use paw_rust_base::topics::get_topic;
use paw_rust_base::{
    await_signal::await_signal,
    env::runtime_env,
    panic_logger::register_panic_logger,
    topics::{Topic, get_topic_names},
};
use paw_sqlx::config::DatabaseConfig;
use paw_team_logs::{BufferedTeamLogs, TeamLogger};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    register_panic_logger();
    setup_nais_otel().unwrap();
    init_stream_wrapper_metrics();
    let team_logger = BufferedTeamLogs::from_nais_env(100)?;
    run_app(team_logger).await
}

async fn run_app(_team_logger: impl TeamLogger + Clone + 'static) -> Result<(), Box<dyn Error>> {
    let kafka_config: KafkaConfig = read_toml_config(read_config_file!("kafka_config.toml"))?;
    let database_config: DatabaseConfig =
        read_toml_config(read_config_file!("database_config.toml"))?;
    let app_state = Arc::new(simple_app_state::AppState::new());
    let http_server_task = spawn_health_server(app_state.clone());
    let hwm_version = *kafka_config.hwm_version;
    let runtime_env = runtime_env();
    let topics = get_topic_names(
        &runtime_env,
        &[
            Topic::Periode,
            Topic::Opplysninger,
            Topic::Profilering,
            Topic::PaaVegneAv,
            Topic::Bekreftelse,
            Topic::Hendelselogg,
            Topic::BekreftelseHendelseLogg,
            Topic::Egenvurdering,
        ],
    );
    let pg_pool = paw_sqlx::postgres::init_db(database_config).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;
    let (tx, rx) = mpsc::unbounded_channel::<TopicPartitionUpdate>();
    let topic_priorities = TopicPriorityList::new(vec![
        (
            get_topic(&runtime_env, &Topic::BekreftelseHendelseLogg).to_string(),
            0,
        ),
        (
            get_topic(&runtime_env, &Topic::Hendelselogg).to_string(),
            10,
        ),
        (get_topic(&runtime_env, &Topic::Periode).to_string(), 50),
        (
            get_topic(&runtime_env, &Topic::Opplysninger).to_string(),
            60,
        ),
        (get_topic(&runtime_env, &Topic::Profilering).to_string(), 70),
    ]);
    // Må bygges før kafka_config flyttes inn i consumeren.
    let stream_config = PawKafkaStreamConfig::from_kafka_config(&kafka_config, topic_priorities)?;
    let consumer = create_kafka_consumer_with_sender(
        app_state.clone(),
        pg_pool.clone(),
        kafka_config,
        &topics,
        tx,
    )?;
    let stream = PawKafkaConsumerStream::new(rx, consumer, pg_pool.clone(), stream_config);
    let kafka_task = tokio::spawn({
        let state = app_state.clone();
        let pg_pool = pg_pool.clone();
        async move {
            let message_processor = InternkontrollMessageProcessor {};
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
    }
    app_state.set_is_alive(false);
    let _ = pg_pool.close().await;
    info!("Pg pool lukket");
    Ok(())
}
