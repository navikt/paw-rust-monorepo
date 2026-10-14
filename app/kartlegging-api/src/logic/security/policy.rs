use crate::api::query_request::shared_query_request;
use axum::extract::Request;
use errors::access::AccessError;
use oauth2::principal::Principal;
use paw_error_handling::problem_details::ProblemDetails;
use paw_tilgangskontroll::model::Policy;
use paw_tilgangskontroll_client::client::PawTilgangskontrollClient;
use std::sync::Arc;

pub struct KartleggingPolicy {
    _client: Arc<PawTilgangskontrollClient>,
}

impl KartleggingPolicy {
    pub fn new(client: Arc<PawTilgangskontrollClient>) -> Self {
        Self { _client: client }
    }
}

impl Policy for KartleggingPolicy {
    async fn evaluate(&self, request: Request) -> Result<Request, ProblemDetails> {
        let path = request.uri().path().to_owned();
        match request.extensions().get::<Principal>() {
            None => {
                tracing::warn!("No principal found");
                Err(ProblemDetails::forbidden(
                    &path,
                    AccessError::MissingPrincipal,
                ))
            }
            Some(principal) => match principal {
                Principal::NavAnsatt(_nav_ansatt) => {
                    let (parts, body) = request.into_parts();
                    let _query = shared_query_request(&parts)?;
                    tracing::debug!("OBO Access");
                    Ok(Request::from_parts(parts, body))
                }
                Principal::NavSystem(_nav_system) => {
                    let (parts, body) = request.into_parts();
                    let _query = shared_query_request(&parts)?;
                    tracing::debug!("M2M Access");
                    Ok(Request::from_parts(parts, body))
                }
                _ => {
                    tracing::warn!("Incorrect principal");
                    Err(ProblemDetails::forbidden(
                        &path,
                        AccessError::IncorrectPrincipal,
                    ))
                }
            },
        }
    }
}
