use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AzureAdM2MClientError {
    #[error("Failed to send token request for scope {scope:?}: {source}")]
    Request {
        scope: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("Token error for scope {scope:?}: HTTP {status} - {error}: {error_description}")]
    TokenError {
        status: u16,
        scope: String,
        error: String,
        error_description: String,
    },
    #[error("Failed to deserialize token response for scope {scope:?}: {source}")]
    Deserialization {
        scope: String,
        #[source]
        source: reqwest::Error,
    },
}
