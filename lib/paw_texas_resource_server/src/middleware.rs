use crate::model::{IntrospectRequest, IntrospectResponse};
use crate::state::AuthState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use errors::auth::AuthError;
use jsonwebtoken::decode_header;
use oauth2::claim::{EntraIdClaims, IdPortenClaims, MaskinportenClaims, TokenXClaims};
use oauth2::issuer::IdentityProvider;
use oauth2::principal::AsPrincipal;
use oauth2::token::{extract_bearer_token, peek_issuer};
use paw_error_handling::problem_details::ProblemDetails;
use std::sync::Arc;

#[tracing::instrument]
pub async fn texas_middleware(
    State(state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let start = std::time::Instant::now();
    let path = request.uri().path().to_string();
    tracing::event!(
        tracing::Level::INFO,
        path = path,
        elapsed = 0i64,
        "Kjører OAuth2-middleware"
    );

    let token = extract_bearer_token(&request)?;
    let header = decode_header(token)
        .map_err(|_| AuthError::InvalidToken("Kunne ikke tolke header".to_string()))?;
    let kid = header.kid.ok_or(AuthError::InvalidToken(
        "Mangler 'kid' header claim".to_string(),
    ))?;
    let alg = header.alg;

    tracing::info!(
        "Finner token issuer for token med KID '{}' og ALG '{:?}'",
        kid,
        alg
    );

    let peeked_iss = peek_issuer(token)?;

    tracing::info!("Validerer token fra issuer '{}'", peeked_iss);

    let identity_provider = match state.config.identity_provider(&peeked_iss) {
        None => return Err(AuthError::InvalidIssuer.into()),
        Some(ip) => ip,
    };
    let introspection_endpoint = state
        .config
        .texas
        .introspection_endpoint
        .clone()
        .into_inner();

    let request_body = IntrospectRequest::new(identity_provider.as_ref(), token.to_string());
    let response = state
        .http_client
        .post(introspection_endpoint)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            ProblemDetails::unauthorized(
                path.to_string(),
                AuthError::IntrospectionFailed(e.to_string()),
            )
        })?;

    let response_status = response.status();
    let response_body = response.text().await.map_err(|e| {
        ProblemDetails::unauthorized(
            path.to_string(),
            AuthError::IntrospectionFailed(e.to_string()),
        )
    })?;
    let introspect_response = serde_json::from_str::<IntrospectResponse>(response_body.as_str())
        .map_err(|e| {
            ProblemDetails::unauthorized(
                path.to_string(),
                AuthError::IntrospectionFailed(e.to_string()),
            )
        })?;

    let was_success = response_status.is_success();
    let was_active_token = introspect_response.active;

    if was_success && was_active_token {
        let principal = match identity_provider {
            IdentityProvider::TokenX => {
                let claims =
                    serde_json::from_str::<TokenXClaims>(response_body.as_str()).map_err(|e| {
                        ProblemDetails::unauthorized(
                            path.to_string(),
                            AuthError::InvalidToken(e.to_string()),
                        )
                    })?;
                claims.as_principal()?
            }
            IdentityProvider::EntraId => {
                let claims = serde_json::from_str::<EntraIdClaims>(response_body.as_str())
                    .map_err(|e| {
                        ProblemDetails::unauthorized(
                            path.to_string(),
                            AuthError::InvalidToken(e.to_string()),
                        )
                    })?;
                claims.as_principal()?
            }
            IdentityProvider::IdPorten => {
                let claims = serde_json::from_str::<IdPortenClaims>(response_body.as_str())
                    .map_err(|e| {
                        ProblemDetails::unauthorized(
                            path.to_string(),
                            AuthError::InvalidToken(e.to_string()),
                        )
                    })?;
                claims.as_principal()?
            }
            IdentityProvider::Maskinporten => {
                let claims = serde_json::from_str::<MaskinportenClaims>(response_body.as_str())
                    .map_err(|e| {
                        ProblemDetails::unauthorized(
                            path.to_string(),
                            AuthError::InvalidToken(e.to_string()),
                        )
                    })?;
                claims.as_principal()?
            }
        };

        let elapsed = format!("{}ms", start.elapsed().as_millis());
        tracing::event!(
            tracing::Level::INFO,
            path = path,
            elapsed = elapsed,
            "Fullførte OAuth2-middleware"
        );
        tracing::debug!("Successful authentication for principal: {:?}", principal);
        request.extensions_mut().insert(principal);
        Ok(next.run(request).await)
    } else if was_success && !was_active_token {
        let elapsed = format!("{}ms", start.elapsed().as_millis());
        tracing::event!(
            tracing::Level::INFO,
            path = path,
            elapsed = elapsed,
            "Fullførte OAuth2-middleware med invalid-token-error"
        );
        Err(AuthError::InvalidToken("Gyldighet er utløpt".to_string()).into())
    } else {
        let elapsed = format!("{}ms", start.elapsed().as_millis());
        tracing::event!(
            tracing::Level::INFO,
            path = path,
            elapsed = elapsed,
            "Fullførte OAuth2-middleware med token-introspection-error"
        );
        let error = introspect_response
            .error
            .unwrap_or("Ukjent feil".to_string());
        Err(AuthError::IntrospectionFailed(error).into())
    }
}
