use crate::model::error::problem_details::ProblemDetails;
use crate::model::dto::request::QueryRequest;
use crate::model::dto::response::{
    Arbeidssoeker, Bekreftelse, Egenvurdering, Profilering, TilknyttetKontor,
};
use axum::response::IntoResponse;
use axum::Json;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "PAW Arbeidssøkerregisteret Oversikt API",
        description = "API for å hente oversikt over arbeidssøkere. Kalles av saksbehandlere via Azure AD.",
        version = "1.0.0",
    ),
    paths(crate::api::oversikt::finn_oversikt),
    components(schemas(
        QueryRequest,
        Arbeidssoeker,
        Bekreftelse,
        Profilering,
        Egenvurdering,
        TilknyttetKontor,
        ProblemDetails,
    )),
    tags(
        (name = "oversikt", description = "Henting av arbeidssøkeroversikt"),
    ),
)]
pub struct ApiDoc;

pub(crate) async fn api_docs() -> impl IntoResponse {
    Json(ApiDoc::openapi())
}
