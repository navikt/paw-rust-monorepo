use crate::serde::error::{AvroSerdeError, enum_type_not_found};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = AvroSerdeError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Endringstype {
    Opprettet,
    Korrigert,
    Annullert,
    Opphoert,
}

/// Utvid denne ved behov. Den er en streng i kontrakten, så den er ikke strengt enumerert.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr,
)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = AvroSerdeError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Opplysningstype {
    AdressebeskyttelseV1,
    DeltBostedV1,
    DoedfoedtBarnV1,
    DoedsfallV1,
    FalskIdV1,
    FamilierelasjonV1,
    FoedselV1,
    FoedselsdatoV1,
    ForelderbarnrelasjonV1,
    SivilstandV1,
    VergemaalEllerFremtidsfullmaktV1,
    UtflyttingFraNorge,
    InnflyttingTilNorge,
    FolkeregisteridentifikatorV1,
    NavnV1,
    SikkerhetstiltakV1,
    StatsborgerskapV1,
    TelefonnummerV1,
    KontaktadresseV1,
    BostedsadresseV1,
    OppholdsadresseV1,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}
