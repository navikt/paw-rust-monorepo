use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureAdM2MConfig {
    pub token_endpoint_url: String,
    pub client_id: String,
    pub client_secret: String,
}

impl AzureAdM2MConfig {
    /// Load configuration from NAIS environment variables:
    /// - `AZURE_OPENID_CONFIG_TOKEN_ENDPOINT`
    /// - `AZURE_APP_CLIENT_ID`
    /// - `AZURE_APP_CLIENT_SECRET`
    pub fn from_env() -> Result<Self, std::env::VarError> {
        Ok(Self {
            token_endpoint_url: std::env::var("AZURE_OPENID_CONFIG_TOKEN_ENDPOINT")?,
            client_id: std::env::var("AZURE_APP_CLIENT_ID")?,
            client_secret: std::env::var("AZURE_APP_CLIENT_SECRET")?,
        })
    }
}
