use crate::config::{AuthConfig, IssuerConfig, HTTP_TIMEOUT, JWKS_MIN_REFRESH_INTERVAL, JWKS_TTL};
use crate::oidc::{fetch_jwks, JwksCache};
use errors::app::AppError;
use errors::auth::AuthError;
use jsonwebtoken::DecodingKey;
use paw_error_handling::problem_details::ProblemDetails;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct IssuerState {
    pub cache: RwLock<JwksCache>,
    pub well_known_url: String,
    pub expected_issuer: String,
    pub client_id: String,
    pub http_client: Client,
}

impl IssuerState {
    pub async fn new(
        http_client: Client,
        issuer_config: Option<IssuerConfig>,
    ) -> Result<Option<Self>, AppError> {
        match issuer_config {
            Some(config) => {
                let well_known_url = config.well_known_url.into_inner();
                let client_id = config.client_id.into_inner();
                let jwks = fetch_jwks(&well_known_url, &http_client)
                    .await
                    .map_err(|e| AppError::AppInitFailed(e.to_string()))?;
                Ok(Some(Self {
                    cache: RwLock::new(jwks.cache),
                    well_known_url,
                    expected_issuer: jwks.issuer,
                    client_id,
                    http_client: http_client.clone(),
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey, ProblemDetails> {
        let needs_refresh = {
            let cache = self.cache.read().await;
            cache.fetched_at.elapsed() > JWKS_TTL || !cache.keys.contains_key(kid)
        };

        if needs_refresh {
            let mut cache = self.cache.write().await;
            let stale = cache.fetched_at.elapsed() > JWKS_TTL;
            let kid_missing = !cache.keys.contains_key(kid);
            let can_refresh = cache.fetched_at.elapsed() > JWKS_MIN_REFRESH_INTERVAL;

            if (stale || kid_missing) && can_refresh {
                let jwks = fetch_jwks(&self.well_known_url, &self.http_client).await?;
                *cache = jwks.cache;
            } else if kid_missing && !can_refresh {
                return Err(AuthError::InvalidToken("Kan ikke oppdatere cache".to_string()).into());
            }
        }

        self.cache
            .read()
            .await
            .keys
            .get(kid)
            .cloned()
            .ok_or(AuthError::InvalidToken("Kunne ikke hente token fra cache".to_string()).into())
    }
}

#[derive(Debug)]
pub struct AuthState {
    pub azure: Option<IssuerState>,
    pub tokenx: Option<IssuerState>,
    pub idporten: Option<IssuerState>,
    pub maskinporten: Option<IssuerState>,
}

impl AuthState {
    pub async fn new(config: AuthConfig) -> Result<Arc<Self>, AppError> {
        let azure_config = config.issuers.azure;
        let tokenx_config = config.issuers.tokenx;
        let idporten_config = config.issuers.idporten;
        let maskinporten_config = config.issuers.maskinporten;
        if azure_config.is_none()
            && tokenx_config.is_none()
            && idporten_config.is_none()
            && maskinporten_config.is_none()
        {
            return Err(AppError::MissingConfig("".to_string()));
        }

        let http_client = Client::builder()
            .timeout(HTTP_TIMEOUT)
            .build()
            .map_err(|_| AppError::AppInitFailed("Kunne ikke opprette HTTP-klient".to_string()))?;

        let azure_issuer_state = IssuerState::new(http_client.clone(), azure_config).await?;
        let tokenx_issuer_state = IssuerState::new(http_client.clone(), tokenx_config).await?;
        let idporten_issuer_state = IssuerState::new(http_client.clone(), idporten_config).await?;
        let maskinporten_issuer_state =
            IssuerState::new(http_client.clone(), maskinporten_config).await?;

        Ok(Arc::new(Self {
            azure: azure_issuer_state,
            tokenx: tokenx_issuer_state,
            idporten: idporten_issuer_state,
            maskinporten: maskinporten_issuer_state,
        }))
    }
}
