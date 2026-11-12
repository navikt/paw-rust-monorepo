use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum ArenaStatus {
    Aktuell,
    Avslag,
    DeltakelseAvbrutt,
    Feilregistrert,
    Fullfort,
    Gjennomfores,
    GjennomforingAvbrutt,
    GjennomforingAvlyst,
    IkkeAktuell,
    IkkeMott,
    Informasjonsmote,
    TakketJaTilTilbud,
    TakketNeiTilTilbud,
    Tilbud,
    Venteliste,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamKometStatus {
    #[serde(rename = "type")]
    pub status: TeamKometStatusType,
    pub aarsak: Option<TeamKometAarsak>,
    pub opprettet_tidspunkt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TeamKometStatusType {
    Avbrutt,
    AvbruttUtkast,
    Deltar,
    Feilregistrert,
    Fullfort,
    HarSluttet,
    IkkeAktuell,
    Kladd,
    PabegyntRegistrering,
    SoktInn,
    UtkastTilPamelding,
    Venteliste,
    VenterPaOppstart,
    Vurderes,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TeamKometAarsak {
    Annet,
    AvlystKontrakt,
    FattJobb,
    Feilregistrert,
    Ferdig,
    FikkIkkePlass,
    IkkeMott,
    OppfyllerIkkeKravene,
    SamarbeidetMedArrangorenErAvbrutt,
    Syk,
    TrengerAnnenStotte,
    Utdanning,
    KravIkkeOppfylt,
    KursFullt,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TeamTiltakStatus {
    Paabegynt,
    ManglerGodkjenning,
    KlarForOppstart,
    Gjennomfores,
    Avsluttet,
    Avbrutt,
    Annullert,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}
