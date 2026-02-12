use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum JobbsituasjonBeskrivelse {
    UkjentVerdi,
    Udefinert,
    HarSagtOpp,
    HarBlittSagtOpp,
    ErPermittert,
    AldriHattJobb,
    IkkeVaertIJobbSiste2Aar,
    AkkuratFullfortUtdanning,
    VilBytteJobb,
    UsikkerJobbsituasjon,
    MidlertidigJobb,
    DeltidsjobVilMer,
    NyJobb,
    Konkurs,
    Annet,
}
