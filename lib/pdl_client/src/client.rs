use crate::config::{PDLClientConfig, BEHANDLINGSNUMMER};
use anyhow::Result;
use graphql_client::{GraphQLQuery, QueryBody};
use pdl_graphql::pdl::hent_person_bolk::HentPersonBolkHentPersonBolk;
use pdl_graphql::pdl::hent_person_navn::HentPersonNavnHentPerson;
use pdl_graphql::pdl::{hent_person_bolk, hent_person_navn, HentPersonBolk, HentPersonNavn};
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;
use tracing::instrument;
use types::identitetsnummer::Identitetsnummer;

#[derive(Clone)]
pub struct PDLClient {
    inner: Arc<PDLClientRef>,
}

#[derive(Clone)]
struct PDLClientRef {
    target_scope: String,
    url: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    behandlingsnummer: String,
}

impl PDLClient {
    pub fn from_config(
        config: PDLClientConfig,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> PDLClient {
        Self::new(
            config.target_scope.into_inner(),
            config.url.into_inner(),
            http_client,
            token_client,
        )
    }

    pub fn new(
        target_scope: String,
        url: String,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
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

    #[instrument(skip(self, identitetsnummer))]
    pub async fn hent_person_bolk(
        &self,
        identitetsnummer: Vec<Identitetsnummer>,
    ) -> Result<Vec<HentPersonBolkHentPersonBolk>> {
        let variables = hent_person_bolk::Variables {
            identer: identitetsnummer.into_iter().map(|id| id.into()).collect(),
            historisk: Some(false),
        };
        let request_body = HentPersonBolk::build_query(variables);
        let response: graphql_client::Response<hent_person_bolk::ResponseData> =
            self.hent_data(request_body).await?;
        if let Some(errors) = response.errors {
            let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(PDLQueryError::UnknownError(messages.join(", ")).into());
        }
        let data = response
            .data
            .ok_or_else(|| PDLQueryError::UnknownError("No data in PDL response".to_string()))?;
        Ok(data.hent_person_bolk)
    }

    #[instrument(skip(self, identitetsnummer))]
    pub async fn hent_person_navn(
        &self,
        identitetsnummer: Identitetsnummer,
    ) -> Result<Option<HentPersonNavnHentPerson>> {
        let variables = hent_person_navn::Variables {
            ident: identitetsnummer.into(),
            historisk: Some(false),
        };
        let request_body = HentPersonNavn::build_query(variables);
        let response: graphql_client::Response<hent_person_navn::ResponseData> =
            self.hent_data(request_body).await?;
        if let Some(errors) = response.errors {
            let messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            Err(PDLQueryError::UnknownError(messages.join(", ")).into())
        } else {
            let data = response.data.ok_or_else(|| {
                PDLQueryError::UnknownError("No data in PDL response".to_string())
            })?;
            Ok(data.hent_person)
        }
    }

    pub async fn hent_data<
        Variables: serde::Serialize,
        ResponseData: serde::de::DeserializeOwned,
    >(
        &self,
        body: QueryBody<Variables>,
    ) -> Result<graphql_client::Response<ResponseData>> {
        let token = match self
            .inner
            .token_client
            .get_token(self.inner.target_scope.clone())
            .await
        {
            Ok(token) => token,
            Err(e) => return Err(e),
        };
        tracing::debug!("Sending request to PDL url: {}", self.inner.url.clone());
        let res = self
            .inner
            .http_client
            .post(self.inner.url.clone())
            .json(&body)
            .bearer_auth(token.access_token)
            .header("Behandlingsnummer", self.inner.behandlingsnummer.clone())
            .send()
            .await?;
        match res.status() {
            reqwest::StatusCode::OK => (),
            reqwest::StatusCode::UNAUTHORIZED => return Err(PDLQueryError::NotAuthorized.into()),
            reqwest::StatusCode::FORBIDDEN => {
                return Err(PDLQueryError::AuthenticationFailed.into());
            }
            _ => {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(PDLQueryError::UnknownError(format!(
                    "PDL query failed with status {}: {}",
                    status, text
                ))
                .into());
            }
        }
        let response: graphql_client::Response<ResponseData> = res.json().await?;
        Ok(response)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PDLQueryError {
    #[error("Not authorized to access this PDL data")]
    NotAuthorized,
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("PDL query error: {0}")]
    UnknownError(String),
}

#[cfg(test)]
mod tests {
    use crate::client::PDLClient;
    use mockito::{Mock, Server, ServerGuard};
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use std::sync::Arc;
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;
    use types::identitetsnummer::Identitetsnummer;

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        client: PDLClient,
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let mock_responses = default_pdl_mock_responses();
            let mut mockito_server = Server::new_async().await;
            let pdl_mock_guard = init_pdl_mock(&mut mockito_server, mock_responses)
                .await
                .expect("Kunne ikke initialisere PDL mock");
            let client = PDLClient::new(
                "test-scope".to_string(),
                format!("{}/pdl", mockito_server.url()),
                reqwest::Client::builder()
                    .no_proxy()
                    .build()
                    .expect("Failed to build reqwest client"),
                Arc::new(TokenClientStub::new()),
            );

            TestContext {
                mockito_server,
                mocks: pdl_mock_guard.mocks,
                client,
            }
        })
        .await
    }

    #[tokio::test]
    async fn test_hent_person() {
        let context = init().await;

        let identitetsnummer = Identitetsnummer::new("01017012345".to_string())
            .expect("Kunne ikke opprette Identitetsnummer");
        let response = context
            .client
            .hent_person_navn(identitetsnummer)
            .await
            .expect("Kunne ikke hente response");

        assert!(response.is_some());
        let response = response.expect("Ingen response funnet");
        assert_eq!(response.navn.len(), 1);
        let navn = response.navn.first().expect("Ingen navn funnet i response");
        assert_eq!(navn.fornavn, "Ola");
        assert_eq!(navn.mellomnavn, None);
        assert_eq!(navn.etternavn, "Nordmann");
    }

    #[tokio::test]
    async fn test_hent_person_navn() {
        let context = init().await;

        let identitetsnummer = Identitetsnummer::new("01017012345".to_string())
            .expect("Kunne ikke opprette Identitetsnummer");
        let response = context
            .client
            .hent_person_navn(identitetsnummer)
            .await
            .expect("Kunne ikke hente response");

        assert!(response.is_some());
        let response = response.expect("Ingen response funnet");
        assert_eq!(response.navn.len(), 1);
        let navn = response.navn.first().expect("Ingen navn funnet i response");
        assert_eq!(navn.fornavn, "Ola");
        assert_eq!(navn.mellomnavn, None);
        assert_eq!(navn.etternavn, "Nordmann");
    }
}
