use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::time::Duration;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);
pub const JWKS_TTL: Duration = Duration::from_secs(3600);
pub const JWKS_MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct IssuerConfig {
    pub well_known_url: String,
    pub client_id: String,
}

#[derive(Debug, Deserialize)]
pub struct IssuersConfig {
    pub azure: Option<IssuerConfig>,
    pub tokenx: Option<IssuerConfig>,
    pub idporten: Option<IssuerConfig>,
    pub maskinporten: Option<IssuerConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub issuers: IssuersConfig,
}
