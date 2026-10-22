#[derive(Debug, thiserror::Error)]
pub enum PawTilgangskontrollClientError {
    #[error("Ikke autorisert")]
    NotAuthorized,
    #[error("Autentisering feilet")]
    AuthenticationFailed,
    #[error("Ukjent feil: {0}")]
    UnknownError(String),
}
