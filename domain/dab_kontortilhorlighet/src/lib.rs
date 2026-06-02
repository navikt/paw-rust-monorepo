use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/dab-schema.graphql",
    query_path = "graphql/hentKontortilhorligheter.graphql"
)]
pub struct HentKontortilhorligheter;
