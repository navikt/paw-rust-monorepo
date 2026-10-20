#[derive(Debug, thiserror::Error)]
pub enum PoaoTilgangClientError {
    #[error("Ikke autorisert")]
    NotAuthorized,
    #[error("Autentisering feilet")]
    AuthenticationFailed,
    #[error("Ukjent feil: {0}")]
    UnknownError(String),
}
