use eksterne_hendelser::periode::PERIODE_TOPIC;
use errors::database::DatabaseError;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use kartlegging_api::api::build_router;
use kartlegging_api::config::{read_auth_config, read_database_config, read_kafka_config};
use kartlegging_api::server::{async_task_handler, shutdown_signal_task, web_server_task};
use paw_oauth2_resource_server::state::AuthState;
use paw_rdkafka::error::KafkaError;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::postgres::{clear_db, init_db};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    register_panic_logger();
    setup_nais_otel()?;

    let topics = [PERIODE_TOPIC];
    let database_config = read_database_config()?;
    let auth_config = read_auth_config()?;
    let kafka_config = read_kafka_config()?;
    let hwm_version = *kafka_config.hwm_version;

    let app_state = Arc::new(simple_app_state::AppState::new());
    let auth_state = AuthState::new(auth_config)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let pg_pool = init_db(database_config).await?;

    // TODO: Fjern før prodsetting!!!
    clear_db(&pg_pool).await?;

    tracing::info!("Migrerer endringer for databasen");
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .map_err(DatabaseError::MigrateSchema)?;

    /*
        let consumer = create_kafka_consumer(app_state.clone(), pg_pool.clone(), kafka_config, &topics)
            .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?;
        let message_processor = OversiktMessageProcessor::new(pg_pool.clone())?;
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
