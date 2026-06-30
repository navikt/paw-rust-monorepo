use anyhow::Result;
use chrono::TimeDelta;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use paw_app_config::read_config_file;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::hwm_message_processor::hwm_process_message;
use paw_rust_base::error::ServerError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::config::DatabaseConfig;
use paw_sqlx::postgres::{clear_db, init_db};
use rdkafka::Message;
use std::num::NonZeroU16;
use std::{sync::Arc, time::Duration};
use texas_client::token_client::create_token_client;
use tokio::{
    signal::{unix::signal, unix::SignalKind},
    task::{JoinError, JoinHandle},
};
use utgang::consumer_function::UtgangMessageProcessor;
use utgang::kafka::kafka_consumer::create_kafka_consumer;
use utgang::kafka::periode_processor::PeriodeProcessorError::ProcessingError;
use utgang::kontroll::{start_kontroll_task, KontrollTask};
use utgang::pdl::pdl_config::PDLClientConfig;
use utgang::pdl::pdl_query::PDLClient;
use utgang::pdl_oppdatering::{start_pdl_oppdatering_task, PdlDataOppdatering};
use utgang::{ARBEIDSSOKERPERIODER_TOPIC, HENDELSELOGG_TOPIC};

const PDL_BATCH_SIZE: NonZeroU16 = NonZeroU16::new(1000).expect("Batch size must be non-zero u16");
const KONTROLL_BATCH_SIZE: NonZeroU16 =
    NonZeroU16::new(200).expect("Batch size must be non-zero u16");

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
    let app_state = Arc::new(simple_app_state::AppState::new());
    let health_routes = axum_health::routes(app_state.clone());
    let web_server_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, health_routes).await?;
        Ok(())
    });
    let db_config = toml::from_str::<DatabaseConfig>(read_config_file!("database_config.toml"))?;
    let pg_pool = init_db(db_config).await?;

    // TODO: Fjern før prodsetting!!!
    clear_db(&pg_pool).await?;

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
    let pdl_pool = pg_pool.clone();
    let consumer_pool = pg_pool.clone();
    let consumer_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        loop {
            let msg = consumer.recv().await?;
            let msg = msg.detach();
            hwm_process_message(hwm_version, consumer_pool.clone(), &msg, &utgang_processor)
                .await
                .map_err(|e| ProcessingError {
                    message: e.to_string(),
                    topic: msg.topic().to_string(),
                    partition: msg.partition(),
                    offset: msg.offset(),
                })?;
        }
    });
    let pdl_client_config =
        toml::from_str::<PDLClientConfig>(read_config_file!("pdl_config.toml"))?;
    tracing::info!("Lastet pdl config: {:?}", pdl_client_config);
    let pdl_client =
        PDLClient::from_config(pdl_client_config, reqwest_client.clone(), token_client);
    let pdl_oppdatering = PdlDataOppdatering::new(
        pdl_pool.clone(),
        pdl_client,
        PDL_BATCH_SIZE,
        TimeDelta::hours(24),
    );
    let pdl_oppdatering_task = start_pdl_oppdatering_task(pdl_oppdatering, Duration::from_mins(1));
    let regelsett = regler_arbeidssoeker::regelsett_v4::regelsett_v4();
    let kontroll = KontrollTask::new(pg_pool.clone(), KONTROLL_BATCH_SIZE, regelsett);
    let kontroll_task = start_kontroll_task(kontroll, Duration::from_mins(5));
    let signal_task = get_shutdown_signal();
    app_state.set_has_started(true);
    tokio::select! {
        res = web_server_task      => haandter_task_resultat("Webserver", res),
        res = consumer_task        => haandter_task_resultat("KafkaConsumer", res),
        res = pdl_oppdatering_task => haandter_task_resultat("PDLOppdatering", res),
        res = kontroll_task        => haandter_task_resultat("Kontroll", res),
        signal = signal_task => {
            tracing::info!("Mottok shutdown-signal: {}", signal?);
            Ok(())
        },
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

fn haandter_task_resultat(navn: &str, res: Result<Result<()>, JoinError>) -> Result<()> {
    match res {
        Ok(Ok(())) => {
            tracing::info!("{} avsluttet normalt", navn);
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::error!("{} avsluttet med feil: {}", navn, e);
            Err(ServerError::InternalProcessTerminated {
                process: navn.to_string(),
                message: e.to_string(),
            }
            .into())
        }
        Err(e) => {
            tracing::error!("Feil i spawned task for {}: {}", navn, e);
            Err(ServerError::InternalProcessTerminated {
                process: navn.to_string(),
                message: e.to_string(),
            }
            .into())
        }
    }
}
