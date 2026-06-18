use serde::Deserialize;
use serde_env_field::env_field_wrap;
use std::collections::HashMap;
use std::time::Duration;

pub const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct TexasConfig {
    pub introspection_endpoint: String,
}

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct IssuerConfig {
    pub issuer: String,
    pub identity_provider: String,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub texas: TexasConfig,
    pub issuers: Vec<IssuerConfig>,
}

impl AuthConfig {
    pub fn identity_provider(&self, issuer: &String) -> Option<String> {
        let identity_providers: HashMap<String, String> = self
            .issuers
            .iter()
            .map(|issuer| {
                (
                    issuer.issuer.clone().into_inner(),
                    issuer.identity_provider.clone().into_inner(),
                )
            })
            .collect();
        identity_providers.get(issuer).cloned()
    }
}
