use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::bekreftelse::Bekreftelse;
use crate::model::dto::egenvurdering::Egenvurdering;
use crate::model::dto::kontor::TilknyttetKontor;
use crate::model::dto::opplysninger::Opplysninger;
use crate::model::dto::periode::Periode;
use crate::model::dto::profilering::Profilering;
use crate::model::dto::request::QueryRequest;
use axum::response::IntoResponse;
use axum::Json;
use paw_error_handling::problem_details::ProblemDetails;
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
        Periode,
        Opplysninger,
        Profilering,
        Egenvurdering,
        Bekreftelse,
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
