#[derive(Debug, thiserror::Error)]
pub enum PawKeyGenClientError {
    #[error("Ikke autorisert")]
    NotAuthorized,
    #[error("Autentisering feilet")]
    AuthenticationFailed,
    #[error("Ukjent feil: {0}")]
    UnknownError(String),
}
