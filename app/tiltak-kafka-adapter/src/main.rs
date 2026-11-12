use errors::database::DatabaseError;
use nais_schema_registry::config::create_schema_registry_settings;
use paw_kafka::error::KafkaError;
use paw_kafka::hwm::kafka_connection::create_kafka_consumer;
use paw_observability::health::simple_app_state::AppState;
use paw_observability::otel::setup_otel;
use paw_rust_base::axum::server::{
    async_task_handler, shutdown_handler, shutdown_signal_task, web_server_task,
};
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::postgres::init_db;
use std::sync::Arc;
use tiltak_kafka_adapter::api::build_router;
use tiltak_kafka_adapter::config::{
    read_app_config, read_database_config, read_kafka_config, read_otel_tracing_config,
};
use tiltak_kafka_adapter::kafka::task::kafka_consumer_task;
use tiltak_kafka_adapter::logic::metrics::task::metrics_task;
use tiltak_kafka_adapter::logic::process::root::RootProcessor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    register_panic_logger();

    let app_config = Arc::new(read_app_config()?);
    let otel_tracing_config = read_otel_tracing_config()?;
    let database_config = read_database_config()?;
    let kafka_config = read_kafka_config()?;
    let schema_registry_settings = create_schema_registry_settings()?;

    let app_state = Arc::new(AppState::new());

    setup_otel(otel_tracing_config)?;

    let pg_pool = init_db(&database_config).await?;

    tracing::info!("Migrerer endringer for databasen");
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .map_err(DatabaseError::MigrateSchema)?;

    let consumer = Arc::new(
        create_kafka_consumer(
            app_state.clone(),
            pg_pool.clone(),
            kafka_config.clone(),
            app_config.clone().kafka.all_topics().as_slice(),
        )
        .map_err(|e| KafkaError::CreateConsumer(e.to_string()))?,
    );
    let processor = RootProcessor::new(app_config.clone(), schema_registry_settings)?;
    let consumer_task = kafka_consumer_task(
        *kafka_config.clone().hwm_version,
        pg_pool.clone(),
        consumer.clone(),
        processor,
    );

    let metrics_task = metrics_task(app_config.clone(), pg_pool.clone());

    let router = build_router(app_state.clone());
    let server_task = web_server_task(router).await;

    let signal_task = shutdown_signal_task();

    app_state.set_has_started(true);

    tokio::select! {
        result = server_task => async_task_handler("Webserver", result),
        result = consumer_task => async_task_handler("KafkaConsumer", result),
        result = metrics_task => async_task_handler("Metrics", result),
        signal = signal_task => shutdown_handler(signal),
    }?;

    pg_pool.close().await;

    Ok(())
}
