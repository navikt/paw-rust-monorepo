#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Felt '{0}' har feil lengde: {1}")]
    StrengLengde(String, i64),
    #[error("Felt '{0}' har feil størrelse: {1}")]
    TallStoerelse(String, i64),
}
