mod consumer_function;
mod db_ops;
mod kafka;
mod pdl;
mod vo;

use crate::consumer_function::UtgangMessageProcessor;
use crate::kafka::hwm_message_processor::{hwm_process_message, MessageProcessor, ProcessorError};
use crate::kafka::kafka_consumer::create_kafka_consumer;
use crate::kafka::periode_processor::PeriodeProcessorError::ProcessingError;
use crate::pdl::pdl_config::PDLClientConfig;
use crate::pdl::pdl_query::PDLClient;
use anyhow::Result;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use paw_app_config::read_config_file;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rust_base::env;
use paw_rust_base::error::ServerError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::config::DatabaseConfig;
use paw_sqlx::postgres::init_db;
use rdkafka::Message;
use std::sync::Arc;
use texas_client::token_client::create_token_client;
use tokio::{
    signal::{unix::signal, unix::SignalKind},
    task::JoinHandle,
};

pub const HENDELSELOGG_TOPIC: &str = "paw.arbeidssoker-hendelseslogg-v1";
pub const ARBEIDSSOKERPERIODER_TOPIC: &str = "paw.arbeidssokerperioder-v1";

#[tokio::main]
async fn main() -> Result<()> {
    register_panic_logger();
    setup_nais_otel()?;
    let reqwest_client = reqwest::Client::new();
    let token_client_config = toml::from_str(read_config_file!("token_client_config.toml"))?;
    let token_client = Arc::new(create_token_client(
        token_client_config,
        reqwest_client.clone(),
    ));
    let pdl_client_config = PDLClientConfig::from_default_file()?;
    let pdl_client =
        PDLClient::from_config(pdl_client_config, reqwest_client.clone(), token_client);
    let app_state = Arc::new(simple_app_state::AppState::new());
    let health_routes = axum_health::routes(app_state.clone());
    let web_server_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, health_routes).await?;
        Ok(())
    });
    let db_config = toml::from_str::<DatabaseConfig>(read_config_file!("database_config.toml"))?;
    let pg_pool = init_db(db_config).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;
    let kafka_config = toml::from_str::<KafkaConfig>(read_config_file!("kafka_config.toml"))?;
    let hwm_version = *kafka_config.hwm_version;
    let consumer = create_kafka_consumer(
        app_state.clone(),
        pg_pool.clone(),
        kafka_config,
        &[HENDELSELOGG_TOPIC, ARBEIDSSOKERPERIODER_TOPIC],
    )
    .map_err(|e| ServerError::InternalProcessTerminated {
        process: "KafkaConsumer".to_string(),
        message: e.to_string(),
    })?;
    let utgang_processor = UtgangMessageProcessor::new()?;
    let consumer_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let processor = utgang_processor;
        loop {
            let msg = consumer.recv().await?;
            let msg = msg.detach();
            hwm_process_message(hwm_version, pg_pool.clone(), &msg, &processor)
                .await
                .map_err(|e| ProcessingError {
                    message: e.to_string(),
                    topic: msg.topic().to_string(),
                    partition: msg.partition(),
                    offset: msg.offset(),
                })?;
        }
    });
    let signal_task = get_shutdown_signal();
    app_state.set_has_started(true);
    tokio::select! {
        res = web_server_task => {
            match res {
                Ok(Ok(())) => {
                    tracing::info!("Webserveren avsluttet normalt");
                    Ok(())
                },
                Ok(Err(e)) => {
                    tracing::error!("Webserveren avsluttet med feil: {}", e);
                    Err(ServerError::InternalProcessTerminated {
                        process: "Webserver".to_string(),
                        message: e.to_string(),
                    })
                },
                Err(e) => {
                    tracing::error!("Feil i spawned task for webserver: {}", e);
                    Err(ServerError::InternalProcessTerminated {
                        process: "Webserver".to_string(),
                        message: e.to_string(),
                    })
                }
            }
        },
        signal = signal_task => {
            let signal = signal?;
            tracing::info!("Mottok shutdown-signal: {}", signal);
            Ok(())
        },
        kafka_consumer = consumer_task => {
            match kafka_consumer {
                Ok(Ok(())) => {
                    tracing::info!("Kafka-consumer avsluttet normalt");
                    Ok(())
                },
                Ok(Err(e)) => {
                    tracing::error!("Kafka-consumer avsluttet med feil: {}", e);
                    Err(ServerError::InternalProcessTerminated {
                        process: "KafkaConsumer".to_string(),
                        message: e.to_string(),
                    })
                },
                Err(e) => {
                    tracing::error!("Feil i spawned task for Kafka-consumer: {}", e);
                    Err(ServerError::InternalProcessTerminated {
                        process: "KafkaConsumer".to_string(),
                        message: e.to_string(),
                    })
                }
            }
        }
    }?;
    Ok(())
}

async fn get_shutdown_signal() -> Result<String> {
    let mut term_signal = signal(SignalKind::terminate())?;
    let mut interrupt_signal = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term_signal.recv() => Ok("SIGTERM".to_string()),
        _ = interrupt_signal.recv() => Ok("SIGINT".to_string())
    }
}
