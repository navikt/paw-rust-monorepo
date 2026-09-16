use std::collections::HashMap;
use std::pin::Pin;
use std::{sync::Arc, time::Duration};

use axum_health::spawn_health_server;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use paw_app_config::{config::read_toml_config, read_config_file};
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::{
    hwm_message_processor::{MessageProcessor, ProcessorError, hwm_process_message},
    kafka_connection::create_kafka_consumer,
    rebalance::rebalance_message::RebalanceMessage,
    stream::{paw_kafka_stream::PawKafkaStream, stream_wrapper::PawKafkaConsumerStream},
};
use paw_rust_base::{
    await_signal::await_signal,
    env::runtime_env,
    panic_logger::register_panic_logger,
    topics::{Topic, get_topic_names},
};
use paw_sqlx::config::DatabaseConfig;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};
use std::error::Error;
use tokio::sync::{Mutex, mpsc};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    register_panic_logger();
    setup_nais_otel().unwrap();
    run_app().await
}

async fn run_app() -> Result<(), Box<dyn Error>> {
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
    let consumer =
        create_kafka_consumer(app_state.clone(), pg_pool.clone(), kafka_config, &topics)?;
    let (_tx, rx) = mpsc::unbounded_channel::<RebalanceMessage>();
    let stream = PawKafkaConsumerStream::new(rx, consumer, Duration::from_millis(10));
    let kafka_task = tokio::spawn({
        let state = app_state.clone();
        let pg_pool = pg_pool.clone();
        async move {
            let message_processor = MultiplexerTestMessageProcessor {
                stream_time: Arc::new(Mutex::new(HashMap::new())),
            };
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

pub struct MultiplexerTestMessageProcessor {
    stream_time: Arc<Mutex<HashMap<i32, i64>>>,
}

impl MessageProcessor for MultiplexerTestMessageProcessor {
    fn process_message<'a>(
        &'a self,
        _: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(async move {
            process(self.stream_time.clone(), msg).await;
            Ok::<(), ProcessorError>(())
        })
    }
}

pub async fn process(map: Arc<Mutex<HashMap<i32, i64>>>, msg: &OwnedMessage) {
    let key = msg.partition();
    let mut stream_time = map.lock().await;
    let current_stream_time = stream_time.get(&key).cloned().unwrap_or(0);
    let record_timestamp = msg.timestamp().to_millis().unwrap_or(-1);
    if (record_timestamp < 0) || (current_stream_time < 0) {
        tracing::warn!(
            "partition {} => undefined timestamp, current stream time: {}, caused by topic: {}",
            msg.partition(),
            current_stream_time,
            msg.topic(),
        );
    } else if record_timestamp > current_stream_time {
        stream_time.insert(key, record_timestamp);
    } else {
        tracing::warn!(
            "partition {} => back in time: {}ms, caused by topic: {}",
            msg.partition(),
            record_timestamp - current_stream_time,
            msg.topic(),
        );
    }
}
