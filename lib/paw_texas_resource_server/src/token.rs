use crate::claim::IssClaim;
use axum::extract::Request;
use axum::http::header;
use errors::auth::AuthError;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

pub fn extract_bearer_token(request: &Request) -> Result<&str, AuthError> {
    request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AuthError::MissingToken)
}

pub fn peek_issuer(token: &str, alg: Algorithm) -> Result<String, AuthError> {
    let mut validation = Validation::new(alg);
    validation.insecure_disable_signature_validation();
    validation.set_required_spec_claims::<String>(&[]);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;

    decode::<IssClaim>(token, &DecodingKey::from_secret(&[]), &validation)
        .map(|data| data.claims.iss)
        .map_err(|e| {
            AuthError::InvalidToken(format!("Kunne ikke trekke ut 'iss' claim pga {}", &e))
        })
}

pub fn validate_token<C: for<'de> Deserialize<'de>>(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<C, AuthError> {
    let mut validation = Validation::new(alg);
    validation.set_audience(&[client_id]);
    validation.set_issuer(&[issuer]);
    validation.set_required_spec_claims(&["exp", "iss", "aud"]);
    validation.validate_exp = true;
    validation.validate_nbf = true;
    validation.validate_aud = true;

    decode::<C>(token, key, &validation)
        .map(|data| data.claims)
        .map_err(|e| {
            AuthError::InvalidToken(format!(
                "Kunne ikke trekke ut ett av ['exp', 'iss', 'aud'] claims pga {}",
                &e
            ))
        })
}
