use std::sync::Arc;
use graphql_client::GraphQLQuery;
use texas_client::{M2MTokenClient};
use crate::hent_person_bolk::{hent_person_bolk, HentPersonBolk};
use crate::hent_person_bolk::hent_person_bolk::HentPersonBolkHentPersonBolk;
use anyhow::Result;
use crate::pdl::pdl_config::{PDLClientConfig, BEHANDLINGSNUMMER};

#[derive(Clone)]
pub struct PDLClient {
    inner: Arc<PDLClientRef>,
}

#[derive(Clone)]
struct PDLClientRef {
    target_scope: String,
    url: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient>,
    behandlingsnummer: String,
}

impl PDLClient {

    pub fn from_config(
        config: PDLClientConfig,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient>,
    ) -> PDLClient {
        Self::new(config.target_scope, config.url, http_client, token_client)
    }
    
    pub fn new(
        target_scope: String,
        url: String,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient>,
    ) -> PDLClient {
        let inner = Arc::new(PDLClientRef {
            target_scope,
            url,
            http_client,
            token_client,
            behandlingsnummer: BEHANDLINGSNUMMER.to_string(),
        });
        PDLClient { inner }
    }
}

impl PDLClient {
    async fn perform_hent_person_bolk(
        &self,
        identitetsnummer: Vec<String>,
    ) -> Result<Vec<HentPersonBolkHentPersonBolk>> {
        let variables = hent_person_bolk::Variables {
            identer: identitetsnummer,
            historisk: Some(true),
        };
        let request_body = HentPersonBolk::build_query(variables);
        let token = match self
            .inner
            .token_client
            .get_token(self.inner.target_scope.clone())
            .await
        {
            Ok(token) => token,
            Err(e) => return Err(e),
        };
        let res = self
            .inner
            .http_client
            .post(self.inner.url.clone())
            .json(&request_body)
            .bearer_auth(token.access_token)
            .header("Behandlingsnummer", self.inner.behandlingsnummer.clone())
            .send()
            .await?;
        match res.status() {
            reqwest::StatusCode::OK => (),
            reqwest::StatusCode::UNAUTHORIZED => return Err(PDLQueryError::NotAuthorized.into()),
            reqwest::StatusCode::FORBIDDEN => return Err(PDLQueryError::AuthenticationFailed.into()),
            _ => {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(PDLQueryError::UnknownError(format!(
                    "PDL query failed with status {}: {}",
                    status, text
                )).into());
            }
        }
        let personer: hent_person_bolk::ResponseData = res.json().await?;
        Ok(personer.hent_person_bolk)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PDLQueryError {
    #[error("Not authorized to access this PDL data")]
    NotAuthorized,
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Unknown error occurred")]
    UnknownError(String),
}
