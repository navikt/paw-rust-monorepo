use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ServerError {
    #[error("Server failed to start")]
    Start,
    #[error("Server received shutdown signal")]
    ShutdownSignal,
    #[error("Server could not start threads")]
    ThreadSpawn,
    #[error("Environment variable '{0}' not found")]
    EnvVarNotFound(String),
}
