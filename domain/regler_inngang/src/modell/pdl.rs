use chrono::{DateTime, Local, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub foedselsdato: Vec<Foedselsdato>,
    pub statsborgerskap: Vec<Statsborgerskap>,
    pub opphold: Vec<Opphold>,
    pub folkeregisterpersonstatus: Vec<Folkeregisterpersonstatus>,
    pub bostedsadresse: Vec<Bostedsadresse>,
    pub innflytting_til_norge: Vec<InnflyttingTilNorge>,
    pub utflytting_fra_norge: Vec<UtflyttingFraNorge>,
}

impl Default for Person {
    fn default() -> Self {
        Self {
            foedselsdato: vec![],
            statsborgerskap: vec![],
            opphold: vec![],
            folkeregisterpersonstatus: vec![],
            bostedsadresse: vec![],
            innflytting_til_norge: vec![],
            utflytting_fra_norge: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Foedselsdato {
    pub foedselsdato: Option<NaiveDate>,
    pub foedselsaar: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statsborgerskap {
    pub land: String,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Opphold {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opphold_fra: Option<DateTime<Local>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opphold_til: Option<DateTime<Local>>,
    #[serde(rename = "type", default = "Oppholdstillatelse::default")]
    pub type_: Oppholdstillatelse,
    pub metadata: Metadata,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Oppholdstillatelse {
    Midlertidig,
    Permanent,
    OpplysningMangler,
    #[serde(other, rename = "__UNKNOWN_VALUE")]
    #[default]
    UnknownValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folkeregisterpersonstatus {
    pub forenklet_status: String,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bostedsadresse {
    pub angitt_flyttedato: Option<NaiveDate>,
    pub gyldig_fra_og_med: Option<NaiveDateTime>,
    pub gyldig_til_og_med: Option<NaiveDateTime>,
    pub vegadresse: Option<Vegadresse>,
    pub matrikkeladresse: Option<Matrikkeladresse>,
    pub ukjent_bosted: Option<UkjentBosted>,
    pub utenlandsk_adresse: Option<UtenlandskAdresse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vegadresse {
    pub kommunenummer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrikkeladresse {
    pub kommunenummer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UkjentBosted {
    pub bostedskommune: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtenlandskAdresse {
    pub landkode: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnflyttingTilNorge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folkeregistermetadata: Option<Folkeregistermetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtflyttingFraNorge {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utflyttingsdato: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folkeregistermetadata: Option<Folkeregistermetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folkeregistermetadata {
    pub gyldighetstidspunkt: Option<DateTime<Local>>,
    pub ajourholdstidspunkt: Option<DateTime<Local>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub endringer: Vec<Endring>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Endring {
    #[serde(rename = "type", default = "Endringstype::default")]
    pub type_: Endringstype,
    pub registrert: DateTime<Local>,
    pub kilde: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Endringstype {
    Opprett,
    Korrigert,
    Opphoer,
    #[serde(other, rename = "__UNKNOWN_VALUE")]
    #[default]
    UnknownValue,
}
