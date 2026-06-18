use crate::model::IntrospectRequest;
use crate::principal::{Anonym, Principal};
use crate::state::AuthState;
use crate::token::{extract_bearer_token, peek_issuer};
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use errors::auth::AuthError;
use jsonwebtoken::decode_header;
use paw_error_handling::problem_details::ProblemDetails;
use std::sync::Arc;

#[tracing::instrument]
pub async fn texas_middleware(
    State(state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let path = request.uri().path();
    let token = extract_bearer_token(&request)?;
    let header = decode_header(token)
        .map_err(|_| AuthError::InvalidToken("Kunne ikke tolke header".to_string()))?;
    let kid = header.kid.ok_or(AuthError::InvalidToken(
        "Mangler 'kid' header claim".to_string(),
    ))?;
    let alg = header.alg;

    tracing::info!("Finner token issuer for token med KID '{}'", kid);

    let peeked_iss = peek_issuer(token, alg)?;

    tracing::info!("Validerer token fra issuer '{}'", peeked_iss);

    let identity_provider = match state.config.identity_provider(&peeked_iss) {
        None => return Err(AuthError::UnknownIssuer.into()),
        Some(ip) => ip,
    };
    let introspection_endpoint = state
        .config
        .texas
        .introspection_endpoint
        .clone()
        .into_inner();

    let request_body = IntrospectRequest::new(identity_provider, token.to_string());
    let response = state
        .http_client
        .post(introspection_endpoint)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            ProblemDetails::unauthorized(path.to_string(), AuthError::InvalidToken(e.to_string()))
        })?;

    let principal: Option<Principal> = if response.status().is_success() {
        /*let response_body = response.json::<IntrospectResponse>().await.map_err(|e| {
            ProblemDetails::unauthorized(path, AuthError::InvalidToken(e.to_string()))
        })?;*/
        let response_body = response.text().await.map_err(|e| {
            ProblemDetails::unauthorized(path.to_string(), AuthError::InvalidToken(e.to_string()))
        })?;
        tracing::info!("TEXAS RESPONSE: {}", response_body);
        Some(Principal::Anonym(Anonym))
    } else {
        let response_body = response.text().await.map_err(|e| {
            ProblemDetails::unauthorized(path.to_string(), AuthError::InvalidToken(e.to_string()))
        })?;
        tracing::error!("TEXAS RESPONSE: {}", response_body);
        None
    };

    if let Some(p) = principal {
        request.extensions_mut().insert(p);
        Ok(next.run(request).await)
    } else {
        Err(AuthError::UnknownIssuer.into())
    }
}
