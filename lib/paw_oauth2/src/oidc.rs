use std::collections::HashMap;
use std::string::ToString;
use std::time::Instant;

use errors::auth::AuthError;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::DecodingKey;
use reqwest::Client;
use serde::Deserialize;

const DEBUG_HASHMAP: &str = "{}";

pub struct JwksCache {
    pub keys: HashMap<String, DecodingKey>,
    pub fetched_at: Instant,
}

impl std::fmt::Debug for JwksCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwksCache")
            .field("keys", &&DEBUG_HASHMAP)
            .field("fetched_at", &self.fetched_at)
            .finish()
    }
}

#[derive(Debug)]
pub struct Jwks {
    pub issuer: String,
    pub cache: JwksCache,
}

#[derive(Debug, Deserialize)]
pub struct OidcDiscovery {
    issuer: String,
    jwks_uri: String,
}

pub async fn fetch_jwks(well_known_url: &str, client: &Client) -> Result<Jwks, AuthError> {
    tracing::info!("Henter JWKS fra {}", well_known_url);
    let oidc_response = client
        .get(well_known_url)
        .send()
        .await
        .map_err(|e| AuthError::OidcFetchFailed(e.to_string()))?;
    let oidc_response = oidc_response
        .error_for_status()
        .map_err(|e| AuthError::OidcFetchFailed(e.to_string()))?;
    let discovery: OidcDiscovery = oidc_response
        .json()
        .await
        .map_err(|e| AuthError::OidcFetchFailed(e.to_string()))?;
    let jwks_response = client
        .get(&discovery.jwks_uri)
        .send()
        .await
        .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
    let jwks_response = jwks_response
        .error_for_status()
        .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
    let jwks: JwkSet = jwks_response
        .json()
        .await
        .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;

    let keys: HashMap<String, DecodingKey> = jwks
        .keys
        .iter()
        .filter_map(|jwk| {
            let kid = jwk.common.key_id.clone()?;
            let key = DecodingKey::from_jwk(jwk).ok()?;
            Some((kid, key))
        })
        .collect();

    if keys.is_empty() {
        return Err(AuthError::NoValidKeysFound);
    }

    Ok(Jwks {
        issuer: discovery.issuer,
        cache: JwksCache {
            keys,
            fetched_at: Instant::now(),
        },
    })
}
