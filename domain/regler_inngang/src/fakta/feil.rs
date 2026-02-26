#[derive(Debug, thiserror::Error)]
pub enum FaktaFeil {
    #[error("Personen har flere fødselsdatoer enn forventet: {0}")]
    FlereFoedselsdatoer(usize),
    #[error("Personen har flere bostedsadresse enn forventet: {0}")]
    FlereBostedsadresse(usize),
}
