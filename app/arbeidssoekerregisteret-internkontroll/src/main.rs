use std::sync::Arc;

use paw_kafka::hwm::stream::stream_wrapper::init_stream_wrapper_metrics;
use paw_kafka::kafka_config::KafkaConfig;
use paw_kafka::signing::RecordVerifier;
use paw_observability::health::simple_app_state::AppState;
use paw_observability::otel::setup_nais_otel;
use paw_rust_base::{config::read_toml_config, read_config_file};
use paw_rust_base::{env::runtime_env, panic_logger::register_panic_logger};
use paw_sqlx::config::DatabaseConfig;
use std::error::Error;

use arbeidssoekerregisteret_internkontroll::core::app_context::AppContext;
use arbeidssoekerregisteret_internkontroll::core::app_runner::run_app;
use arbeidssoekerregisteret_internkontroll::features::regler::config::RegelConfig;
use arbeidssoekerregisteret_internkontroll::reset_db::reset_db_if_needed;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    register_panic_logger();
    setup_nais_otel().unwrap();
    init_stream_wrapper_metrics();
    let record_verifier = RecordVerifier::from_embedded_keys()?;
    let kafka_config: KafkaConfig = read_toml_config(read_config_file!("kafka_config.toml"))?;
    let database_config: DatabaseConfig =
        read_toml_config(read_config_file!("database_config.toml"))?;
    let regel_config: RegelConfig = read_toml_config(read_config_file!("regel_config.toml"))?;
    let pg_pool = paw_sqlx::postgres::init_db(&database_config).await?;
    reset_db_if_needed(&pg_pool, *kafka_config.hwm_version).await?;
    let app_context = AppContext::new(
        kafka_config,
        record_verifier,
        Arc::new(AppState::new()),
        runtime_env(),
        pg_pool,
        regel_config,
    );
    run_app(app_context).await
}
