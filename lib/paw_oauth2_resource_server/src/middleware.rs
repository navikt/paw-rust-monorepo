use crate::state::AuthState;
use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use errors::auth::AuthError;
use jsonwebtoken::decode_header;
use oauth2::principal::{
    build_azure_principal, build_idporten_principal, build_maskinporten_principal,
    build_tokenx_principal,
};
use oauth2::token::{extract_bearer_token, peek_issuer};
use paw_error_handling::problem_details::ProblemDetails;
use std::pin::Pin;
use std::sync::Arc;

pub fn oauth2_middleware(state: Arc<AuthState>) -> OAuth2MiddlewareLayer {
    type F = fn(State<Arc<AuthState>>, Request, Next) -> BoxFut<Result<Response, ProblemDetails>>;
    axum::middleware::from_fn_with_state(state, oauth2_auth_handler_boxed as F)
}

type BoxFut<T> = Pin<Box<dyn Future<Output = T> + Send>>;

pub type OAuth2MiddlewareLayer = axum::middleware::FromFnLayer<
    fn(State<Arc<AuthState>>, Request, Next) -> BoxFut<Result<Response, ProblemDetails>>,
    Arc<AuthState>,
    (State<Arc<AuthState>>, Request),
>;

fn oauth2_auth_handler_boxed(
    state: State<Arc<AuthState>>,
    request: Request,
    next: Next,
) -> BoxFut<Result<Response, ProblemDetails>> {
    Box::pin(oauth2_auth_handler(state, request, next))
}

#[tracing::instrument]
pub async fn oauth2_auth_handler(
    State(state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let start = std::time::Instant::now();
    let path = request.uri().path();
    tracing::event!(
        tracing::Level::INFO,
        path = path,
        elapsed = "0ms",
        "Kjører OAuth2-middleware"
    );

    let token = extract_bearer_token(&request)?;
    let header = decode_header(token)
        .map_err(|_| AuthError::InvalidToken("Kunne ikke tolke header".to_string()))?;
    let kid = header.kid.ok_or(AuthError::InvalidToken(
        "Mangler 'kid' header claim".to_string(),
    ))?;
    let alg = header.alg;

    tracing::debug!(
        "Finner token issuer for token med KID '{}' og ALG '{:?}'",
        kid,
        alg
    );

    let peeked_iss = peek_issuer(token)?;

    tracing::debug!("Tolker og validerer token fra issuer '{}'", peeked_iss);

    let mapped_principal = if let Some(tokenx_state) = &state.tokenx {
        if peeked_iss == tokenx_state.expected_issuer {
            let key = tokenx_state.get_decoding_key(&kid).await?;
            Some(build_tokenx_principal(
                token,
                alg,
                &key,
                &tokenx_state.expected_issuer,
                &tokenx_state.client_id,
            )?)
        } else {
            None
        }
    } else if let Some(azure_state) = &state.azure {
        if peeked_iss == azure_state.expected_issuer {
            let key = azure_state.get_decoding_key(&kid).await?;
            Some(build_azure_principal(
                token,
                alg,
                &key,
                &azure_state.expected_issuer,
                &azure_state.client_id,
            )?)
        } else {
            None
        }
    } else if let Some(idporten_state) = &state.idporten {
        if peeked_iss == idporten_state.expected_issuer {
            let key = idporten_state.get_decoding_key(&kid).await?;
            Some(build_idporten_principal(
                token,
                alg,
                &key,
                &idporten_state.expected_issuer,
                &idporten_state.client_id,
            )?)
        } else {
            None
        }
    } else if let Some(maskinporten_state) = &state.maskinporten {
        if peeked_iss == maskinporten_state.expected_issuer {
            let key = maskinporten_state.get_decoding_key(&kid).await?;
            Some(build_maskinporten_principal(
                token,
                alg,
                &key,
                &maskinporten_state.expected_issuer,
                &maskinporten_state.client_id,
            )?)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(principal) = mapped_principal {
        let elapsed = format!("{}ms", start.elapsed().as_millis());
        tracing::event!(
            tracing::Level::INFO,
            path = path,
            elapsed = elapsed,
            "Fullførte OAuth2-middleware"
        );
        request.extensions_mut().insert(principal);
        Ok(next.run(request).await)
    } else {
        let elapsed = format!("{}ms", start.elapsed().as_millis());
        tracing::event!(
            tracing::Level::ERROR,
            path = path,
            elapsed = elapsed,
            "Fullførte OAuth2-middleware med unknown-issuer-error"
        );
        Err(AuthError::UnknownIssuer.into())
    }
}
