use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::client::opprett_oppgave_request::PrioritetV1;

#[derive(Debug, Deserialize, Serialize)]
pub struct OppgaveDto {
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personident: Option<String>,
    pub tildelt_enhetsnr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endret_av_enhetsnr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opprettet_av_enhetsnr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journalpost_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandles_av_applikasjon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saksreferanse: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aktoer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orgnr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tilordnet_ressurs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beskrivelse: Option<String>,
    pub tema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandlingstema: Option<String>,
    pub oppgavetype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandlingstype: Option<String>,
    pub versjon: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappe_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opprettet_av: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endret_av: Option<String>,
    pub prioritet: PrioritetV1,
    pub status: OppgavestatusDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frist_ferdigstillelse: Option<String>,
    pub aktiv_dato: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opprettet_tidspunkt: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ferdigstilt_tidspunkt: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endret_tidspunkt: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bruker: Option<BrukerDto>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BrukerDto {
    pub ident: String,
    pub type_: BrukertypeDto,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum BrukertypeDto {
    #[serde(rename = "PERSON")]
    Person,
    #[serde(rename = "ARBEIDSGIVER")]
    Arbeidsgiver,
    #[serde(rename = "SAMHANDLER")]
    Samhandler,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum OppgavestatusDto {
    #[serde(rename = "OPPRETTET")]
    Opprettet,
    #[serde(rename = "AAPNET")]
    Aapnet,
    #[serde(rename = "UNDER_BEHANDLING")]
    UnderBehandling,
    #[serde(rename = "FERDIGSTILT")]
    Ferdigstilt,
    #[serde(rename = "FEILREGISTRERT")]
    Feilregistrert,
}
