use graphql_client::GraphQLQuery;

use crate::pdl::hent_person_bolk::HentPersonBolkHentPersonBolk;

type Date = String;
type DateTime = String;
pub type PdlPerson = HentPersonBolkHentPersonBolk;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/pdl-schema.graphql",
    query_path = "graphql/hentPersonBolk.graphql"
)]
pub struct HentPersonBolk;
