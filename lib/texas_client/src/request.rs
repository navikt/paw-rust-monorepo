use paw_rust_base::env_var::{EnvVarNotFoundError, get_env};

const ENTRA_ID: &str = "entra_id";
const NAIS_TOKEN_ENDPOINT_ENV: &str = "NAIS_TOKEN_ENDPOINT";

pub fn get_nais_token_endpoint() -> Result<String, EnvVarNotFoundError> {
    get_env(NAIS_TOKEN_ENDPOINT_ENV)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct M2MTokenRequest {
    identity_provider: &'static str,
    target: String,
}

impl M2MTokenRequest {
    pub fn new(target: String) -> Self {
        Self {
            identity_provider: ENTRA_ID,
            target: target,
        }
    }
}
