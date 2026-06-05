use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{AsRefStr, EnumString};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct OversiktResponse {
    pub arbeidssoekere: Vec<Arbeidssoeker>,
    pub paging: PagingResponse,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PagingResponse {
    pub page: i32,
    pub page_size: i32,
    pub total_items: i64,
    pub sort_order: SortOrder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr, ToSchema)]
pub enum SortOrder {
    #[strum(serialize = "ASC")]
    #[serde(rename = "ASC")]
    Ascending,
    #[strum(serialize = "DESC")]
    #[serde(rename = "DESC")]
    Descending,
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, ToSchema)]
pub struct Arbeidssoeker {
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub fornavn: String,
    pub mellomnavn: Option<String>,
    pub etternavn: String,
    pub ledig_siden: Option<DateTime<Utc>>,
    pub periode: Periode,
    pub opplysninger: Option<Opplysninger>,
    pub profilering: Option<Profilering>,
    pub egenvurdering: Option<Egenvurdering>,
    pub bekreftelse: Option<Bekreftelse>,
    pub bekreftelse_paa_vegne_av: Vec<Bekreftelsesloesning>,
    pub tilknyttet_kontor: Vec<TilknyttetKontor>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, ToSchema)]
pub struct Periode {
    pub id: Uuid,
    pub startet: DateTime<Utc>,
    pub avsluttet: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Opplysninger {
    pub id: Uuid,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Profilering {
    pub id: Uuid,
    pub profilert_til: ProfilertTil,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Egenvurdering {
    pub id: Uuid,
    pub egenvurdert_til: ProfilertTil,
    pub tidspunkt: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Bekreftelse {
    pub id: Uuid,
    pub gjelder_fra: DateTime<Utc>,
    pub gjelder_til: DateTime<Utc>,
    pub har_jobbet: bool,
    pub vil_fortsette: bool,
    pub bekreftelsesloesning: Bekreftelsesloesning,
}
#[derive(Debug, Serialize, ToSchema)]
pub struct TilknyttetKontor {
    pub kontor_id: String,
    pub kontor_navn: String,
    pub kontor_type: String,
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr, ToSchema,
)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfilertTil {
    Udefinert,
    AntattGodeMuligheter,
    AntattBehovForVeiledning,
    OppgittHindringer,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

impl fmt::Display for ProfilertTil {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr, ToSchema)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Bekreftelsesloesning {
    UkjentVerdi,
    Arbeidssoekerregisteret,
    Dagpenger,
    FriskmeldtTilArbeidsformidling,
}

impl fmt::Display for Bekreftelsesloesning {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}

fn enum_type_not_found(type_: &str) -> EnumTypeParseError {
    EnumTypeParseError::UkjentType(type_.to_string())
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EnumTypeParseError {
    #[error("Ukjent enum: {0}")]
    UkjentType(String),
}
