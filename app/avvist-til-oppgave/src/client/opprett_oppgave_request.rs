use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct OpprettOppgaveRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personident: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orgnr: Option<String>,
    pub tildelt_enhetsnr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opprettet_av_enhetsnr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub journalpost_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandles_av_applikasjon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saksreferanse: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beskrivelse: Option<String>,
    pub tema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandlingstema: Option<String>,
    pub oppgavetype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandlingstype: Option<String>,
    pub aktiv_dato: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frist_ferdigstillelse: Option<String>,
    pub prioritet: PrioritetV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

//TODO: Fjern eller forenkle når vi vet mer om hva vi trenger
impl Default for OpprettOppgaveRequest {
    fn default() -> Self {
        Self {
            personident: None,
            orgnr: None,
            tildelt_enhetsnr: String::new(),
            opprettet_av_enhetsnr: None,
            journalpost_id: None,
            behandles_av_applikasjon: None,
            saksreferanse: None,
            beskrivelse: None,
            tema: String::new(),
            behandlingstema: None,
            oppgavetype: String::new(),
            behandlingstype: None,
            aktiv_dato: chrono::Utc::now().format("%Y-%m-%d").to_string(),
            frist_ferdigstillelse: None,
            prioritet: PrioritetV1::Norm,
            uuid: None,
        }
    }
}


#[derive(Debug, Deserialize, Serialize)]
pub enum PrioritetV1 {
    #[serde(rename = "HOY")]
    Hoy,
    #[serde(rename = "NORM")]
    Norm,
    #[serde(rename = "LAV")]
    Lav,
}
