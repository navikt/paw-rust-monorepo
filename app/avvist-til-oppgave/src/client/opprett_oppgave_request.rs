use crate::domain::oppgave::Oppgave;
use chrono::Utc;
use serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OpprettOppgaveRequest {
    pub aktiv_dato: String,
    pub prioritet: PrioritetV1,
    pub tema: String,
    pub oppgavetype: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub personident: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orgnr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samhandlernr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tildelt_enhetsnr: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandlingstema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behandlingstype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frist_ferdigstillelse: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

pub fn to_oppgave_request(oppgave: &Oppgave) -> OpprettOppgaveRequest {
    OpprettOppgaveRequest {
        personident: Some(oppgave.identitetsnummer.clone()),
        aktiv_dato: Utc::now().format("%Y-%m-%d").to_string(),
        oppgavetype: "KONT_BRUK".to_string(),
        prioritet: PrioritetV1::Norm,
        tema: "GEN".to_string(),
        ..Default::default()
    }
}

impl Default for OpprettOppgaveRequest {
    fn default() -> Self {
        Self {
            tema: String::new(),
            oppgavetype: String::new(),
            aktiv_dato: Utc::now().format("%Y-%m-%d").to_string(),
            prioritet: PrioritetV1::Norm,
            personident: None,
            orgnr: None,
            samhandlernr: None,
            tildelt_enhetsnr: None,
            opprettet_av_enhetsnr: None,
            journalpost_id: None,
            behandles_av_applikasjon: None,
            saksreferanse: None,
            beskrivelse: None,
            behandlingstema: None,
            behandlingstype: None,
            frist_ferdigstillelse: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::oppgave_status::OppgaveStatus;
    use crate::domain::oppgave_type::OppgaveType;
    use chrono::Utc;

    #[test]
    fn test_to_oppgave_request() {
        let identitetsnummer = "12345678901";
        let oppgave = Oppgave {
            identitetsnummer: identitetsnummer.to_string(),
            ..Default::default()
        };

        let request = to_oppgave_request(&oppgave);

        assert_eq!(request.personident, Some(identitetsnummer.to_string()));
        assert_eq!(request.oppgavetype, "KONT_BRUK");
        assert_eq!(request.tema, "GEN");
        assert!(matches!(request.prioritet, PrioritetV1::Norm));
        assert!(request.orgnr.is_none());
        assert!(request.tildelt_enhetsnr.is_none());
    }

    impl Default for Oppgave {
        fn default() -> Self {
            Self {
                id: 1,
                type_: OppgaveType::AvvistUnder18,
                status: OppgaveStatus::Ubehandlet,
                opplysninger: vec![],
                arbeidssoeker_id: 123,
                identitetsnummer: "12345678910".to_string(),
                ekstern_oppgave_id: None,
                tidspunkt: Utc::now(),
                hendelse_logg: vec![],
            }
        }
    }
}
