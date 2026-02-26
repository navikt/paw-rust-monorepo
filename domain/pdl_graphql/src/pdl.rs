use graphql_client::GraphQLQuery;

type Date = String;
type DateTime = String;
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/pdl-schema.graphql",
    query_path = "graphql/hentPersonBolk.graphql"
)]
pub struct HentPersonBolk;
