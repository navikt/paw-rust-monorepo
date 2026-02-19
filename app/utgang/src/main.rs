mod db_ops;
mod kafka;
mod pdl;
mod vo;

use anyhow::Result;
use std::sync::Arc;

use crate::kafka::kafka_consumer::create_kafka_consumer;
use crate::pdl::pdl_config::PDLClientConfig;
use crate::pdl::pdl_query::PDLClient;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use paw_app_config::read_config_file;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rust_base::env;
use paw_rust_base::error::ServerError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::config::DatabaseConfig;
use paw_sqlx::postgres::init_db;
use tokio::{
    signal::{unix::signal, unix::SignalKind},
    task::JoinHandle,
};

#[tokio::main]
async fn main() -> Result<()> {
    register_panic_logger();
    setup_nais_otel()?;
    let reqwest_client = reqwest::Client::new();
    let token_client = Arc::new(texas_client::token_client::create_token_client(
        reqwest_client.clone(),
    )?);
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
    let ssl =
        env::get_env("NAIS_DATABASE_PAW_ARBEIDSSOEKERREGISTERET_UTGANG_PDL_UTGANGBETA1_SSLKEY")
            .map_err(|e| ServerError::InternalProcessTerminated {
                process: "EnvironmentVariable".to_string(),
                message: e.to_string(),
            })?;
    println!("Using SSL of length: {}", ssl.len());
    let db_config = toml::from_str::<DatabaseConfig>(read_config_file!("database_config.toml"))?;
    let pg_pool = init_db(db_config).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;
    let kafka_config = toml::from_str::<KafkaConfig>(read_config_file!("kafka_config.toml"))?;
    let consumer = create_kafka_consumer(
        app_state.clone(),
        pg_pool.clone(),
        kafka_config,
        &[
            "paw.arbeidssokerperioder-v1",
            "paw.arbeidssoker-hendelseslogg-v1",
        ],
    )
    .map_err(|e| ServerError::InternalProcessTerminated {
        process: "KafkaConsumer".to_string(),
        message: e.to_string(),
    })?;
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
