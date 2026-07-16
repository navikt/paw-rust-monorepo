use crate::config::PawKeyGenClientConfig;
use crate::error::PawKeyGenClientError;
use crate::model::{IdentitetRequest, IdentitetResponse, KeyRequest, KeyResponse};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;

#[derive(Clone)]
pub struct PawKeyGenClient {
    url: String,
    scope: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl PawKeyGenClient {
    pub fn from_config(
        config: PawKeyGenClientConfig,
        http_client: reqwest::Client,
        token_client: Arc<dyn M2MTokenClient + Send + Sync>,
    ) -> PawKeyGenClient {
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
    ) -> PawKeyGenClient {
        PawKeyGenClient {
            url,
            scope,
            http_client,
            token_client,
        }
    }

    pub async fn hent(&self, identitet: String) -> anyhow::Result<KeyResponse> {
        let url = format!("{}/api/v2/hent", self.url);
        let request = KeyRequest { ident: identitet };
        self.post(url, request).await
    }

    pub async fn finn_identiteter(&self, identitet: String) -> anyhow::Result<IdentitetResponse> {
        let url = format!("{}/api/v2/identiteter", self.url);
        let request = IdentitetRequest { identitet };
        self.post(url, request).await
    }

    async fn post<S: Serialize, T: DeserializeOwned>(
        &self,
        url: String,
        request: S,
    ) -> anyhow::Result<T> {
        let token = match self.token_client.get_token(self.scope.clone()).await {
            Ok(token) => token,
            Err(e) => return Err(e),
        };
        let response = self
            .http_client
            .post(url)
            .json(&request)
            .bearer_auth(token.access_token)
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => Ok(response.json().await?),
            reqwest::StatusCode::UNAUTHORIZED => Err(PawKeyGenClientError::NotAuthorized.into()),
            reqwest::StatusCode::FORBIDDEN => {
                Err(PawKeyGenClientError::AuthenticationFailed.into())
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                let error = format!("Kall feilet med status {}: {}", status, text);
                Err(PawKeyGenClientError::UnknownError(error).into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::PawKeyGenClient;
    use crate::model::IdentitetType;
    use kafka_key_gen_mock::{default_kafka_key_gen_mock_responses, init_kafka_key_gen_mock};
    use mockito::{Mock, Server, ServerGuard};
    use std::sync::Arc;
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        client: PawKeyGenClient,
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let mock_responses = default_kafka_key_gen_mock_responses();
            let mut mockito_server = Server::new_async().await;
            let kafka_key_gen_mock_guard =
                init_kafka_key_gen_mock(&mut mockito_server, mock_responses)
                    .await
                    .expect("Kunne ikke initialisere Kafka Key Gen mock");
            let client = PawKeyGenClient::new(
                mockito_server.url(),
                "test-scope".to_string(),
                reqwest::Client::builder()
                    .no_proxy()
                    .build()
                    .expect("Failed to build reqwest client"),
                Arc::new(TokenClientStub::new()),
            );

            TestContext {
                mockito_server,
                mocks: kafka_key_gen_mock_guard.mocks,
                client,
            }
        })
        .await
    }

    #[tokio::test]
    async fn test_finn_identiteter() {
        let context = init().await;

        let response = context
            .client
            .finn_identiteter("01017012345".to_string())
            .await
            .expect("Kunne ikke hente response");

        assert_eq!(response.record_key, Some(-12345));
        assert_eq!(response.arbeidssoeker_id, Some(12345));
        assert_eq!(response.identiteter.len(), 4);
        let aktor_id_liste = response
            .identiteter
            .iter()
            .filter(|i| i.identitet_type == IdentitetType::Aktorid)
            .collect::<Vec<_>>();
        assert_eq!(aktor_id_liste.len(), 1);
        let aktor_id = aktor_id_liste.first().expect("Fant ingen AKTORID");
        assert_eq!(aktor_id.identitet, "101701234500");
        assert_eq!(aktor_id.identitet_type, IdentitetType::Aktorid);
        assert_eq!(aktor_id.gjeldende, true);
        let arbeidssoeker_id_liste = response
            .identiteter
            .iter()
            .filter(|i| i.identitet_type == IdentitetType::Arbeidssoekerid)
            .collect::<Vec<_>>();
        assert_eq!(arbeidssoeker_id_liste.len(), 1);
        let arbeidssoeker_id = arbeidssoeker_id_liste
            .first()
            .expect("Fant ingen ARBEIDSSOEKERID");
        assert_eq!(arbeidssoeker_id.identitet, "12345");
        assert_eq!(
            arbeidssoeker_id.identitet_type,
            IdentitetType::Arbeidssoekerid
        );
        assert_eq!(arbeidssoeker_id.gjeldende, true);
        let folkeregisterident_liste = response
            .identiteter
            .iter()
            .filter(|i| i.identitet_type == IdentitetType::Folkeregisterident)
            .collect::<Vec<_>>();
        assert_eq!(folkeregisterident_liste.len(), 2);
        let folkeregisterident_1 = folkeregisterident_liste
            .get(0)
            .expect("Fant ingen FOLKEREGISTERIDENT");
        let folkeregisterident_2 = folkeregisterident_liste
            .get(1)
            .expect("Fant ingen FOLKEREGISTERIDENT");
        assert_eq!(folkeregisterident_1.identitet, "01017012345");
        assert_eq!(
            folkeregisterident_1.identitet_type,
            IdentitetType::Folkeregisterident
        );
        assert_eq!(folkeregisterident_1.gjeldende, true);
        assert_eq!(folkeregisterident_2.identitet, "41017012345");
        assert_eq!(
            folkeregisterident_2.identitet_type,
            IdentitetType::Folkeregisterident
        );
        assert_eq!(folkeregisterident_2.gjeldende, false);
        assert!(response.pdl_identiteter.is_none());
        assert!(response.konflikter.is_none());
    }
}
