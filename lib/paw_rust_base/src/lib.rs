use crate::env_var::EnvVarNotFoundError;

pub mod env_var;
pub mod error_handling;

pub fn nais_otel_service_name() -> Result<String, EnvVarNotFoundError> {
    env_var::get_env("OTEL_SERVICE_NAME")
}

pub fn nais_namespace() -> Result<String, EnvVarNotFoundError> {
    env_var::get_env("NAIS_NAMESPACE")
}

pub fn git_commit() -> &'static str {
    option_env!("GIT_COMMIT_HASH").unwrap_or("dev-build")
}
