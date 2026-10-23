use crate::config::PawTilgangskontrollV2ClientConfig;
use crate::error::PawTilgangskontrollClientError;
use crate::model::{
    NavAnsattNavEnhetTilgangRequest, NavAnsattPersonTilgangRequest, TilgangResponse, TilgangType,
};
use paw_observability::client::trace_context_aware_headers;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;
use types::identitetsnummer::Identitetsnummer;
use types::nav_enhet_id::NavEnhetId;
use types::nav_ident::NavIdent;

#[derive(Clone)]
pub struct PawTilgangskontrollV2Client {
    url: String,
    scope: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl PawTilgangskontrollV2Client {
    pub fn from_config(
        config: PawTilgangskontrollV2ClientConfig,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> Self {
        Self::new(
            config.url.into_inner(),
            config.target_scope.into_inner(),
            http_client,
            token_client,
        )
    }

    pub fn new(
        url: String,
        scope: String,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> Self {
        Self {
            url,
            scope,
            http_client,
            token_client,
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn nav_ansatt_tilgang_til_person(
        &self,
        tilgang: TilgangType,
        nav_ident: NavIdent,
        identitetsnummer: Identitetsnummer,
    ) -> anyhow::Result<TilgangResponse> {
        let url = format!("{}/api/v1/tilgang/person", self.url);
        let nav_ident_str = nav_ident.as_ref();
        let identitetsnummer_str = identitetsnummer.as_ref();
        let request = NavAnsattPersonTilgangRequest::new(
            tilgang,
            nav_ident_str.to_string(),
            identitetsnummer_str.to_string(),
        );
        self.post(url, request).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn nav_ansatt_tilgang_til_enhet(
        &self,
        nav_ident: NavIdent,
        nav_enhet_id: NavEnhetId,
    ) -> anyhow::Result<TilgangResponse> {
        let url = format!("{}/api/v1/tilgang/enhet", self.url);
        let nav_ident_str = nav_ident.as_ref();
        let nav_enhet_id_str = nav_enhet_id.as_ref();
        let request = NavAnsattNavEnhetTilgangRequest::new(
            nav_ident_str.to_string(),
            nav_enhet_id_str.to_string(),
        );
        self.post(url, request).await
    }

    async fn post<S: Serialize, T: DeserializeOwned>(
        &self,
        url: String,
        request: S,
    ) -> anyhow::Result<T> {
        let headers = trace_context_aware_headers();
        let token = match self.token_client.get_token(self.scope.clone()).await {
            Ok(token) => token,
            Err(e) => return Err(e),
        };
        let response = self
            .http_client
            .post(url)
            .headers(headers)
            .json(&request)
            .bearer_auth(token.access_token)
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(response.json().await?),
            reqwest::StatusCode::UNAUTHORIZED => {
                Err(PawTilgangskontrollClientError::NotAuthorized.into())
            }
            reqwest::StatusCode::FORBIDDEN => {
                Err(PawTilgangskontrollClientError::AuthenticationFailed.into())
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                let error = format!("Kall feilet med status {}: {}", status, text);
                Err(PawTilgangskontrollClientError::UnknownError(error).into())
            }
        }
    }
}
