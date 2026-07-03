use crate::config::AuthConfig;
use errors::app::AppError;
use reqwest::Client;
use std::sync::Arc;

#[derive(Debug)]
pub struct AuthState {
    pub config: AuthConfig,
    pub http_client: Client,
}

impl AuthState {
    pub async fn new(config: AuthConfig, http_client: Client) -> Result<Arc<Self>, AppError> {
        Ok(Arc::new(Self {
            config,
            http_client,
        }))
    }
}
