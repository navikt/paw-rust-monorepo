use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::time::Duration;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub introspection_endpoint: String,
}
