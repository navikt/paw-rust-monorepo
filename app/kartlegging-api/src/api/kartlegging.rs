use crate::logic::query::finn_for_identitetsnummer_v2::finn_for_identitetsnummer_v2;
use crate::logic::query::finn_for_tilknyttet_kontor_v2::finn_for_tilknyttet_kontor_v2;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response_v2::KartleggingResponse;
use crate::model::state::RouterState;
use axum::extract::State;
use axum::Json;
use paw_error_handling::problem_details::ProblemDetails;

#[tracing::instrument(skip(state, request), fields(arbeidssoekere_count))]
pub(crate) async fn finn_kartlegging(
    State(state): State<RouterState>,
    request: String,
) -> anyhow::Result<Json<KartleggingResponse>, ProblemDetails> {
    tracing::debug!("Query request: {}", request);
    let query_request: QueryRequest = serde_json::from_str(&request).map_err(|e| {
        tracing::error!("Feil ved deserialisering av request body: {}", e);
        ProblemDetails::validation_error(
            "/api/v1/kartlegging".to_string(),
            "Ugyldig request body".to_string(),
        )
    })?;
    match query_request {
        QueryRequest::Identitetsnummer(query) => {
            query.validate()?;
            let response = finn_for_identitetsnummer_v2(&state.pg_pool, &query).await?;
            tracing::Span::current().record("query_hit_count", response.arbeidssoekere.len());
            Ok(Json(response))
        }
        QueryRequest::TilknyttetKontor(query) => {
            query.validate()?;
            let response = finn_for_tilknyttet_kontor_v2(&state.pg_pool, &query).await?;
            tracing::Span::current().record("query_hit_count", response.arbeidssoekere.len());
            Ok(Json(response))
        }
    }
}
