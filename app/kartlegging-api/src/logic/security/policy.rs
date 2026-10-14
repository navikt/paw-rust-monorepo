use axum::extract::Request;
use errors::access::AccessError;
use oauth2::principal::Principal;
use paw_error_handling::problem_details::ProblemDetails;
use paw_tilgangskontroll::model::Policy;
use paw_tilgangskontroll_client::client::PawTilgangskontrollClient;
use std::sync::Arc;

pub struct KartleggingPolicy {
    client: Arc<PawTilgangskontrollClient>,
}

impl KartleggingPolicy {
    pub fn new(client: Arc<PawTilgangskontrollClient>) -> Self {
        Self { client }
    }
}

impl Policy for KartleggingPolicy {
    async fn evaluate(&self, request: Request) -> Result<Request, ProblemDetails> {
        let path = request.uri().path().to_owned();
        if request.extensions().is_empty() {
            tracing::trace!("No extensions found");
        } else {
            tracing::trace!("Extensions: {:?}", request.extensions());
        }
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
                    tracing::debug!("OBO Access");
                    Ok(request)
                }
                Principal::NavSystem(_nav_system) => {
                    tracing::debug!("M2M Access");
                    Ok(request)
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
