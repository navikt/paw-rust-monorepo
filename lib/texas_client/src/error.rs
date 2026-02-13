use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TexasClientError {
    #[error("Failed send request for target {target:?}: HTTP {status:?}")]
    Request { status: u16, target: String },
    #[error("Received error response for target {target:?}: HTTP {status:?}")]
    Response { status: u16, target: String },
}
