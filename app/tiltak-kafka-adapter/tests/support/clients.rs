use mulighetsrommet_client::client::MulighetsrommetClient;
use paw_key_gen_client::client::PawKeyGenClient;
use std::sync::Arc;
use texas_client::token_client::M2MTokenClient;
use token_client_stub::TokenClientStub;

pub struct ClientFactory {
    mockito_server_url: String,
    http_client: reqwest::Client,
    token_client: Arc<dyn M2MTokenClient + Send + Sync>,
}

impl ClientFactory {
    pub fn new(mockito_server_url: String) -> Self {
        Self {
            mockito_server_url,
            http_client: reqwest::Client::new(),
            token_client: Arc::new(TokenClientStub::new()),
        }
    }

    pub fn create_key_gen_client(&self) -> Arc<PawKeyGenClient> {
        Arc::new(PawKeyGenClient::new(
            self.mockito_server_url.clone(),
            "key-gen-scope".to_string(),
            self.http_client.clone(),
            self.token_client.clone(),
        ))
    }

    pub fn create_mulighetsrommet_client(&self) -> MulighetsrommetClient {
        MulighetsrommetClient::new(
            self.mockito_server_url.clone(),
            "mulighetsrommet-scope".to_string(),
            self.http_client.clone(),
            self.token_client.clone(),
        )
    }
}
