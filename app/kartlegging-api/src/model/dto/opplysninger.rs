use crate::model::parse::{enum_type_not_found, EnumTypeParseError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use uuid::Uuid;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Opplysninger {
    pub id: Uuid,
    pub jobbsituasjon: Vec<Jobbsituasjon>,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Jobbsituasjon {
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
