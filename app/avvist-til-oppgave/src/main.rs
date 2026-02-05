mod avvist_melding_status;
mod db;
mod hendelse_processor;
mod kafka;
pub mod avvist_hendelse;

use axum_health::routes;
use health_and_monitoring::nais_otel_setup::setup_nais_otel;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::consumer_error::ConsumerError;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rust_base::database_error::DatabaseError;
use paw_rust_base::error_handling::AppError;
use paw_sqlx::init_db;
use std::error::Error;
use std::sync::Arc;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn AppError>> {
    setup_nais_otel()?;
    log::info!("Application started");
    let appstate = Arc::new(AppState::new());
    let health_routes = routes(appstate.clone());
    let web_server_task: JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> =
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
            axum::serve(listener, health_routes).await?;
            Ok(())
        });

    let pg_pool =
        init_db("NAIS_DATABASE_PAW_ARBEIDSSOEKERREGISTERET_AVVIST_TIL_OPPGAVE_AVVISTTILOPPGAVE")
            .await
            .map_err(|err| DatabaseError {
                message: format!("Failed to initialize database: {}", err),
            })?;
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await
        .map_err(|migrate_error| DatabaseError {
            message: format!("Database migration failed: {}", migrate_error),
        })?;

    let kafka_config = KafkaConfig::new("avvist_til_oppgave_v1", "ssl");
    let hendelselogg_topic = &["paw.arbeidssoker-hendelseslogg-v1"];
    let hendelselogg_consumer = kafka::consumer::create(
        appstate.clone(),
        pg_pool.clone(),
        kafka_config,
        hendelselogg_topic,
    )
    .map_err(|err| ConsumerError {
        message: format!("Failed to create Kafka consumer: {}", err),
    })?;

    let _ = hendelse_processor::start_processing_loop(
        hendelselogg_consumer,
        pg_pool.clone(),
        appstate.clone(),
    );

    appstate.set_has_started(true);
    match web_server_task.await {
        Ok(Ok(())) => log::info!("Web server exited successfully."),
        Ok(Err(e)) => log::error!("Web server error: {}", e),
        Err(e) => log::error!("Task join error: {}", e),
    }
    Ok(())
}
