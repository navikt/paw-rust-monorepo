use crate::pdl_query::hent_person_bolk::HentPersonBolkHentPersonBolk;
use graphql_client::{GraphQLQuery, Response};
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;
use texas_client::{M2MTokenClient, token_client};

type Date = String;
type DateTime = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/pdl/pdl-schema.graphql",
    query_path = "graphql/pdl/hentPersonBolk.graphql"
)]
pub struct HentPersonBolk;

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
}

impl PDLClient {
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
        });
        PDLClient { inner }
    }
}

impl PDLClient {
    async fn perform_hent_person_bolk(
        &self,
        identitetsnummer: Vec<String>,
    ) -> Result<Vec<HentPersonBolkHentPersonBolk>, Box<dyn Error>> {
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
            .send()
            .await?;
        let personer: hent_person_bolk::ResponseData = res.json().await?;
        Ok(personer.hent_person_bolk)
    }
}
