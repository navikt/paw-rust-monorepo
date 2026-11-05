use std::sync::Arc;

use paw_kafka::{kafka_config::KafkaConfig, signing::RecordVerifier};
use paw_observability::health::simple_app_state::AppState;
use paw_rust_base::env::RuntimeEnv;
use sqlx::PgPool;

use crate::features::regler::config::RegelConfig;

pub struct AppContext {
    pub kafka_config: KafkaConfig,
    pub record_verifier: RecordVerifier,
    pub app_state: Arc<AppState>,
    pub runtime_env: RuntimeEnv,
    pub pg_pool: PgPool,
    pub regel_config: RegelConfig,
}

impl AppContext {
    pub fn new(
        kafka_config: KafkaConfig,
        record_verifier: RecordVerifier,
        app_state: Arc<AppState>,
        runtime_env: RuntimeEnv,
        pg_pool: PgPool,
        regel_config: RegelConfig,
    ) -> Self {
        Self {
            kafka_config,
            record_verifier,
            app_state,
            runtime_env,
            pg_pool,
            regel_config,
        }
    }
}
