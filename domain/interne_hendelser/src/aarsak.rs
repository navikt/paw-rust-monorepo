use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Aarsak {
    IkkeFunnet,
    Savnet,
    Doed,
    Opphoert,
    Under18Aar,
    IkkeBosattINorgeIHenholdTilFolkeregisterloven,
    UkjentAlder,
    EuEoesStatsborgerMenHarStatusIkkeBosatt,
    BaOmAaAvsluttePeriode,
    RegisterGracePeriodeUtloept,
    RegisterGracePeriodeUtloeptEtterEksternInnsamling,
    TekniskFeilUnderKalkuleringAvAarsak,
    IngenAarsakFunnet,
    #[serde(other)]
    Udefinert,
}

impl Default for Aarsak {
    fn default() -> Self {
        Aarsak::Udefinert
    }
}
