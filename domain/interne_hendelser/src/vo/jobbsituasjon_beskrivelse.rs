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
    #[serde(rename = "IKKE_VAERT_I_JOBB_SISTE_2_AAR")]
    IkkeVaertIJobbSiste2Aar,
    AkkuratFullfortUtdanning,
    VilBytteJobb,
    UsikkerJobbsituasjon,
    MidlertidigJobb,
    DeltidsjobbVilMer,
    NyJobb,
    Konkurs,
    Annet,
}
