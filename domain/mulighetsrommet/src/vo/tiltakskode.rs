use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TeamKometTiltakskode {
    ArbeidsrettetRehabilitering,
    Avklaring,
    DigitaltOppfolgingstiltak,
    Jobbklubb,
    Oppfolging,
    ArbeidsforberedendeTrening,
    VarigTilrettelagtArbeidSkjermet,
    TilrettelagtArbeidOrdinaer,
    Arbeidsmarkedsopplaering,
    #[serde(rename = "ENKELTPLASS_ARBEIDSMARKEDSOPPLAERING")]
    #[strum(serialize = "ENKELTPLASS_ARBEIDSMARKEDSOPPLAERING")]
    EnkelplassArbeidsmarkedsopplaering,
    #[serde(rename = "ENKELTPLASS_FAG_OG_YRKESOPPLAERING")]
    #[strum(serialize = "ENKELTPLASS_FAG_OG_YRKESOPPLAERING")]
    EnkelplassFagOgYrkesopplaering,
    FagOgYrkesopplaering,
    GruppeArbeidsmarkedsopplaering,
    GruppeFagOgYrkesopplaering,
    HoyereUtdanning,
    HoyereYrkesfagligUtdanning,
    NorskopplaeringGrunnleggendeFerdigheterFov,
    Studiespesialisering,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum TeamTiltakTiltakskode {
    Arbeidstrening,
    MidlertidigLonnstilskudd,
    VarigLonnstilskudd,
    Mentor,
    Inkluderingstilskudd,
    Sommerjobb,
    Vtao,
    FirearigLonnstilskudd,
    #[serde(untagged)]
    #[strum(default, transparent)]
    Ukjent(String),
}
