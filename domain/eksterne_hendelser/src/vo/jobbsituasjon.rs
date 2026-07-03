use crate::parse::{enum_type_not_found, EnumTypeParseError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Jobbsituasjon {
    pub beskrivelser: Vec<BeskrivelseMedDetaljer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BeskrivelseMedDetaljer {
    pub beskrivelse: Beskrivelse,
    pub detaljer: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Beskrivelse {
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
    #[serde(other)]
    #[default]
    UkjentVerdi,
}
