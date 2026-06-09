use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Konfigurasjon mangler: {0}")]
    MissingConfig(String),
    #[error("Init av app feilet: {0}")]
    AppInitFailed(String),
}
