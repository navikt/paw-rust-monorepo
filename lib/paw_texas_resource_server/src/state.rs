use crate::config::{AuthConfig, HTTP_TIMEOUT};
use errors::app::AppError;
use reqwest::Client;
use std::sync::Arc;

#[derive(Debug)]
pub struct AuthState {
    pub introspection_endpoint: String,
    pub http_client: Client,
}

impl AuthState {
    pub async fn new(config: AuthConfig) -> Result<Arc<Self>, AppError> {
        let http_client = Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .map_err(|_| AppError::AppInitFailed("Kunne ikke opprette HTTP-klient".to_string()))?;

        Ok(Arc::new(Self {
            introspection_endpoint: config.introspection_endpoint.into_inner(),
            http_client,
        }))
    }
}
