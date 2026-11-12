#[derive(Debug, thiserror::Error)]
pub enum MulighetsrommetClientError {
    #[error("Kunne ikke hente token for Mulighetsrommet")]
    Token(#[source] anyhow::Error),
    #[error("Kall til Mulighetsrommet feilet")]
    Request(#[source] reqwest::Error),
    #[error("Mulighetsrommet svarte med HTTP-status {0}")]
    HttpStatus(reqwest::StatusCode),
    #[error("Kunne ikke lese tiltakshistorikk fra Mulighetsrommet")]
    Response(#[source] reqwest::Error),
}
