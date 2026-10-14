use thiserror::Error;

#[derive(Debug, Error)]
pub enum AccessError {
    #[error("Manglende bruker")]
    MissingPrincipal,
    #[error("Ugyldig bruker")]
    IncorrectPrincipal,
    #[error("Bruker mangler påkrevd rolle: {0}")]
    InsufficientRoles(String),
    #[error("Bruker har ikke tilgang: {0}")]
    AccessDenied(String),
}
