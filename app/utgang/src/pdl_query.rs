use crate::pdl_query::hent_person_bolk::HentPersonBolkHentPersonBolk;
use graphql_client::{GraphQLQuery, Response};
use std::error::Error;
use std::fmt::Debug;
use std::sync::Arc;

type Date = String;
type DateTime = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/pdl/pdl-schema.graphql",
    query_path = "graphql/pdl/hentPersonBolk.graphql"
)]
pub struct HentPersonBolk;

#[derive(Clone, Debug)]
pub struct PDLClient {
    inner: Arc<PDLClientRef>
}

#[derive(Clone, Debug)]
struct PDLClientRef {
    url: String,
    http_client: reqwest::Client,
}

impl PDLClient {
    pub fn new(url: String, http_client: reqwest::Client) -> PDLClient {
        let inner = Arc::new(PDLClientRef { url, http_client });
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
        let res = self.inner.http_client.post("/graphql").json(&request_body).send().await?;
        let personer: hent_person_bolk::ResponseData = res.json().await?;
        Ok(personer.hent_person_bolk)
    }
}
