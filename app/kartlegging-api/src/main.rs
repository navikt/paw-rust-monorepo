use dab_oppfolgingperioder::oppfolgingsperiode::SISTE_OPPFOLGINGSPERIODE_V3_TOPIC;
use eksterne_hendelser::bekreftelse::bekreftelse::BEKREFTELSE_TOPIC;
use eksterne_hendelser::bekreftelse::paa_vegne_av::BEKREFTELSE_PAAVEGNEAV_TOPIC;
use eksterne_hendelser::egenvurdering::EGENVURDERING_TOPIC;
use eksterne_hendelser::opplysninger::OPPLYSNINGER_TOPIC;
use eksterne_hendelser::periode::PERIODE_TOPIC;
use eksterne_hendelser::profilering::PROFILERING_TOPIC;
use errors::app::AppError;
use errors::database::DatabaseError;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use kartlegging_api::api::build_router;
use kartlegging_api::config::{
    read_auth_config, read_database_config, read_kafka_config, read_otel_tracing_config,
    read_paw_key_gen_client_config, read_pdl_client_config, read_token_client_config,
    HTTP_TIMEOUT,
};
use kartlegging_api::kafka::consumer::{create_kafka_consumer, kafka_consumer_task};
use kartlegging_api::logic::process::message_process::KartleggingMessageProcessor;
use kartlegging_api::server::{async_task_handler, shutdown_signal_task, web_server_task};
use paw_key_gen_client::client::PawKeyGenClient;
use paw_oauth2_resource_server::state::AuthState;
use paw_otel_tracing::otel_setup::setup_otel;
use paw_rdkafka::error::KafkaError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::postgres::{clear_db, init_db};
use pdl_client::pdl_query::PDLClient;
use reqwest::Client;
use std::sync::Arc;
use texas_client::token_client::create_token_client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    register_panic_logger();

    let otel_tracing_config = read_otel_tracing_config()?;
    let database_config = read_database_config()?;
    let auth_config = read_auth_config()?;
    let kafka_config = read_kafka_config()?;
    let token_client_config = read_token_client_config()?;
    let key_gen_client_config = read_paw_key_gen_client_config()?;
    let pdl_client_config = read_pdl_client_config()?;

    setup_otel(otel_tracing_config)?;

    let topics = [
        PERIODE_TOPIC,
        OPPLYSNINGER_TOPIC,
        PROFILERING_TOPIC,
        EGENVURDERING_TOPIC,
        BEKREFTELSE_TOPIC,
        BEKREFTELSE_PAAVEGNEAV_TOPIC,
        /*SISTE_OPPFOLGINGSPERIODE_V3_TOPIC,*/
    ];

    let hwm_version = *kafka_config.hwm_version;

    let http_client = Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|_| AppError::AppInitFailed("Kunne ikke opprette HTTP-klient".to_string()))?;

    let app_state = Arc::new(simple_app_state::AppState::new());
    let auth_state = AuthState::new(auth_config, http_client.clone())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let pg_pool = init_db(database_config).await?;

    // TODO: Fjern før prodsetting!!!
    clear_db(&pg_pool).await?;

    tracing::info!("Migrerer endringer for databasen");
    sqlx::migrate!("./migrations_testdata") // TODO: Endre fra migrering med testdata
        .run(&pg_pool)
        .await
        .map_err(DatabaseError::MigrateSchema)?;

    let token_client = Arc::new(create_token_client(
        token_client_config,
        http_client.clone(),
    ));
    let key_gen_client = Arc::new(PawKeyGenClient::from_config(
        key_gen_client_config,
        http_client.clone(),
        token_client.clone(),
    ));
    let pdl_client = Arc::new(PDLClient::from_config(
        pdl_client_config,
        http_client.clone(),
        token_client.clone(),
    ));
/*
    let consumer = create_kafka_consumer(app_state.clone(), pg_pool.clone(), kafka_config, &topics)
        .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?;
    let message_processor =
        KartleggingMessageProcessor::new(key_gen_client.clone(), pdl_client.clone())?;
    let consumer_task =
        kafka_consumer_task(pg_pool.clone(), hwm_version, consumer, message_processor);
*/
    let router = build_router(app_state.clone(), pg_pool.clone(), auth_state);
    let server_task = web_server_task(router).await;

    let signal_task = shutdown_signal_task();

    app_state.set_has_started(true);

    tokio::select! {
        result = server_task => async_task_handler("Webserver", result),
        //result = consumer_task => async_task_handler("KafkaConsumer", result),
        signal = signal_task => {
            tracing::info!("Mottok shutdown-signal: {}", signal?);
            Ok(())
        },
    }?;

    Ok(())
}
