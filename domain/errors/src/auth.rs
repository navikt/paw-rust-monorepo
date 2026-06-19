use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Manglende eller ugyldig Authorization-header")]
    MissingToken,
    #[error("Ugyldig token: {0}")]
    InvalidToken(String),
    #[error("Ugyldig token issuer")]
    InvalidIssuer,
    #[error("Ukjent token issuer")]
    UnknownIssuer,
    #[error("Token inneholdt ikke claim {0}")]
    MissingClaim(String),
    #[error("Kunne ikke hente OIDC info: {0}")]
    OidcFetchFailed(String),
    #[error("Kunne ikke hente JWKS: {0}")]
    JwksFetchFailed(String),
    #[error("Kunne ikke tolke token: {0}")]
    IntrospectionFailed(String),
    #[error("JWKS inneholdt ingen gyldige nøkler")]
    NoValidKeysFound,
}
