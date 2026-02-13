use crate::env_var::EnvVarNotFoundError;

pub mod database_error;
pub mod env_var;
pub mod error_handling;
pub mod panic_logger;
pub mod convenience_functions;

pub fn nais_otel_service_name() -> Result<String, EnvVarNotFoundError> {
    env_var::get_env("OTEL_SERVICE_NAME")
}

pub fn nais_namespace() -> Result<String, EnvVarNotFoundError> {
    env_var::get_env("NAIS_NAMESPACE")
}

pub fn nais_cluster_name() -> Result<String, EnvVarNotFoundError> {
    env_var::get_env("NAIS_CLUSTER_NAME")
}

pub fn git_commit() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("dev-build")
}
