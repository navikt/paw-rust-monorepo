use crate::regler::resultat::Problem;

#[derive(Debug, thiserror::Error)]
pub enum FaktaFeil {
    #[error("Personen har flere fødselsdatoer enn forventet: {0}")]
    FlereFoedselsdatoer(usize),
    #[error("Personen har flere bostedsadresser enn forventet: {0}")]
    FlereBostedsadresser(usize),
    #[error("Personen har flere oppholdstillatelser enn forventet: {0}")]
    FlereOppholdstillatelser(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum EvalueringFeil {
    #[error("Evaluering av regler returnerte feil")]
    Problemer(Vec<Problem>),
}
