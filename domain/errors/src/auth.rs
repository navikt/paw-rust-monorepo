use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Manglende eller ugyldig Authorization-header")]
    MissingToken,
    #[error("Ugyldig token: {0}")]
    InvalidToken(String),
    #[error("Ukjent token issuer")]
    UnknownIssuer,
    #[error("NAVident mangler i Azure-token")]
    MissingNavIdent,
    #[error("pid mangler i TokenX-token")]
    MissingPid,
    #[error("Kunne ikke hente OIDC info: {0}")]
    OidcFetchFailed(String),
    #[error("Kunne ikke hente JWKS: {0}")]
    JwksFetchFailed(String),
    #[error("JWKS inneholdt ingen gyldige nøkler")]
    NoValidKeysFound,
}
