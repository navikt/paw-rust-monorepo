use crate::logic::query;
use crate::model::context::AppContext;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response::OversiktResponse;
use axum::extract::State;
use axum::Json;
use paw_error_handling::problem_details::ProblemDetails;
use tracing::instrument;

#[utoipa::path(
    post,
    path = "/api/v1/oversikt",
    tag = "oversikt",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Oversikt over arbeidssøkere for gitt identitetsnummer", body = OversiktResponse),
        (status = 500, description = "Intern feil", body = ProblemDetails),
    ),
)]
#[instrument(skip(state, request), fields(arbeidssoekere_count))]
pub(crate) async fn finn_oversikt(
    State(state): State<AppContext>,
    Json(request): Json<QueryRequest>,
) -> anyhow::Result<Json<OversiktResponse>, ProblemDetails> {
    request
        .validate()
        .map_err(|e| ProblemDetails::validation_error("/api/v1/oversikt".to_string(), e))?;
    let response = query::finn_oversikt(&state.db, &request).await?;
    tracing::Span::current().record("arbeidssoekere_count", response.arbeidssoekere.len());
    Ok(Json(response))
}
