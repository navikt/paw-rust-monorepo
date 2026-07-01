use crate::logic::query::finn_for_identitetsnummer_v2::finn_for_identitetsnummer_v2;
use crate::logic::query::finn_for_kontortilknytning::finn_for_kontortilknytning;
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
) -> Result<Json<KartleggingResponse>, ProblemDetails> {
    const PATH: &str = "/api/v1/kartlegging";
    let query_request: QueryRequest = serde_json::from_str(&request).map_err(|e| {
        tracing::error!("Feil ved deserialisering av request body: {}", e);
        ProblemDetails::validation_error(PATH.to_string(), "Ugyldig request body".to_string())
    })?;

    let mut tx = state.pg_pool.begin().await.map_err(|e| {
        tracing::error!("Kunne ikke starte transaksjon: {}", e);
        ProblemDetails::database_error(PATH.to_string(), "Transaksjon feilet".to_string())
    })?;

    let response = match query_request {
        QueryRequest::Identitetsnummer(query) => {
            query
                .validate()
                .map_err(|e| ProblemDetails::validation_error(PATH.to_string(), e.to_string()))?;
            finn_for_identitetsnummer_v2(&mut tx, &query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(PATH.to_string(), "Spørring feilet".to_string())
                })?
        }
        QueryRequest::TilknyttetKontor(query) => {
            query
                .validate()
                .map_err(|e| ProblemDetails::validation_error(PATH.to_string(), e.to_string()))?;
            finn_for_kontortilknytning(&mut tx, &query)
                .await
                .map_err(|e| {
                    tracing::error!("Feil ved spørring: {}", e);
                    ProblemDetails::database_error(PATH.to_string(), "Spørring feilet".to_string())
                })?
        }
    };

    tx.commit().await.map_err(|e| {
        tracing::error!("Kunne ikke commite transaksjon: {}", e);
        ProblemDetails::database_error(PATH.to_string(), "Transaksjon feilet".to_string())
    })?;
    tracing::Span::current().record("arbeidssoekere_count", response.arbeidssoekere.len());
    Ok(Json(response))
}
