use oauth2::issuer::IdentityProvider;
use serde::Deserialize;
use serde_env_field::env_field_wrap;

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct TexasConfig {
    pub introspection_endpoint: String,
}

#[env_field_wrap]
#[derive(Debug, Deserialize)]
pub struct IssuerConfig {
    pub issuer: String,
    pub identity_provider: IdentityProvider,
}

#[derive(Debug, Deserialize)]
pub struct AuthConfig {
    pub texas: TexasConfig,
    pub issuers: Vec<IssuerConfig>,
}

impl AuthConfig {
    pub fn identity_provider(&self, iss: &String) -> Option<IdentityProvider> {
        for issuer_config in &self.issuers {
            let issuer = issuer_config.issuer.clone().into_inner();
            let identity_provider = issuer_config.identity_provider.clone().into_inner();
            if issuer.as_str() == iss.as_str() {
                return Some(identity_provider);
            }
        }
        None
    }
}
