use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    #[default]
    Udefinert,
}
