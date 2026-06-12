use crate::logic::query::finn_for_identitetsnummer::finn_for_identitetsnummer;
use crate::logic::query::finn_for_tilknyttet_kontor::finn_for_tilknyttet_kontor;
use crate::model::context::AppContext;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response::OversiktResponse;
use axum::extract::State;
use axum::Json;
use paw_error_handling::problem_details::ProblemDetails;

#[utoipa::path(
    post,
    path = "/api/v1/oversikt",
    tag = "oversikt",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Oversikt over arbeidssøkere for gitt query", body = OversiktResponse),
        (status = 500, description = "Intern feil", body = ProblemDetails),
    ),
)]
#[tracing::instrument(skip(context, request), fields(arbeidssoekere_count))]
pub(crate) async fn finn_oversikt(
    State(context): State<AppContext>,
    request: String,
) -> anyhow::Result<Json<OversiktResponse>, ProblemDetails> {
    tracing::info!("Mottok request: {}", request);
    let query_request: QueryRequest = serde_json::from_str(&request).map_err(|e| {
        tracing::error!("Feil ved deserialisering av request body: {}", e);
        ProblemDetails::validation_error(
            "/api/v1/oversikt".to_string(),
            "Ugyldig request body".to_string(),
        )
    })?;
    match query_request {
        QueryRequest::Identitetsnummer(query) => {
            query.validate()?;
            let response = finn_for_identitetsnummer(&context.db, &query).await?;
            tracing::Span::current().record("arbeidssoekere_count", response.arbeidssoekere.len());
            Ok(Json(response))
        }
        QueryRequest::TilknyttetKontor(query) => {
            query.validate()?;
            let response = finn_for_tilknyttet_kontor(&context.db, &query).await?;
            tracing::Span::current().record("arbeidssoekere_count", response.arbeidssoekere.len());
            Ok(Json(response))
        }
    }
}
