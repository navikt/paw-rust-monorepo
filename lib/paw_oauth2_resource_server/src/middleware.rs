use crate::principal::{
    build_azure_principal, build_idporten_principal, build_maskinporten_principal,
    build_tokenx_principal,
};
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
pub async fn oauth2_middleware(
    State(state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let token = extract_bearer_token(&request)?;
    let header = decode_header(token)
        .map_err(|_| AuthError::InvalidToken("Kunne ikke tolke header".to_string()))?;
    let kid = header.kid.ok_or(AuthError::InvalidToken(
        "Mangler 'kid' header claim".to_string(),
    ))?;
    let alg = header.alg;

    tracing::info!("Finner token issuer for token med KID '{}'", kid);

    let peeked_iss = peek_issuer(token, alg)?;

    tracing::info!("Tolker og validerer token fra issuer '{}'", peeked_iss);

    let principal = if let Some(azure_state) = &state.azure {
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
    } else if let Some(tokenx_state) = &state.tokenx {
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

    if let Some(p) = principal {
        request.extensions_mut().insert(p);
        Ok(next.run(request).await)
    } else {
        Err(AuthError::UnknownIssuer.into())
    }
}
