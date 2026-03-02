use chrono::{DateTime, Local, NaiveDate};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveHendelseMelding {
    pub hendelse: OppgaveHendelse,
    pub utfort_av: OppgaveUtfortAv,
    pub oppgave: OppgaveOppgave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveHendelse {
    pub hendelsestype: OppgaveHendelsetype,
    pub tidspunkt: DateTime<Local>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveUtfortAv {
    pub nav_ident: String,
    pub enhetsnr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveOppgave {
    pub oppgave_id: i64,
    pub versjon: i32,
    pub tilordning: OppgaveTilordning,
    pub kategorisering: OppgaveKategorisering,
    pub behandlingsperiode: OppgaveBehandlingsperiode,
    pub bruker: OppgaveBruker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveTilordning {
    pub enhetsnr: String,
    pub enhetsmappe_id: i64,
    pub nav_ident: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveKategorisering {
    pub tema: String,
    pub oppgavetype: String,
    pub behandlingstema: String,
    pub behandlingstype: String,
    pub prioritet: OppgavePrioritet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveBehandlingsperiode {
    pub aktiv: NaiveDate,
    pub frist: NaiveDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveBruker {
    pub ident: String,
    pub ident_type: OppgaveIdentType,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OppgaveHendelsetype {
    OppgaveOpprettet,
    OppgaveEndret,
    OppgaveFerdigstilt,
    OppgaveFeilregistrert,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OppgavePrioritet {
    Hoy,
    Normal,
    Lav,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OppgaveIdentType {
    #[strum(serialize = "FOLKEREGISTERIDENT")]
    #[serde(rename = "FOLKEREGISTERIDENT")]
    FolkeregisterIdent,
    #[strum(serialize = "NPID")]
    #[serde(rename = "NPID")]
    NpId,
    #[strum(serialize = "ORGNR")]
    #[serde(rename = "ORGNR")]
    OrgNr,
    #[strum(serialize = "SAMHANDLERNR")]
    #[serde(rename = "SAMHANDLERNR")]
    SamhandlerNr,
}
