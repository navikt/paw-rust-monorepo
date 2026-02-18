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

const OPPGAVE_BESKRIVELSE: &str = r#"Personen har forsøkt å registrere seg som arbeidssøker, men er sperret fra å gjøre dette da personen er under 18 år.
For mindreårige arbeidssøkere trengs det samtykke fra begge foresatte for å kunne registrere seg.
Se "Samtykke fra foresatte til unge under 18 år - registrering som arbeidssøker, øvrige tiltak og tjenester".

Når samtykke er innhentet kan du registrere arbeidssøker via flate for manuell registrering i modia."#;

pub fn create_oppgave_request(identitetsnummer: String) -> OpprettOppgaveRequest {
    OpprettOppgaveRequest {
        personident: Some(identitetsnummer),
        aktiv_dato: Utc::now().format("%Y-%m-%d").to_string(),
        oppgavetype: "KONT_BRUK".to_string(),
        prioritet: PrioritetV1::Norm,
        tema: "GEN".to_string(),
        beskrivelse: Some(OPPGAVE_BESKRIVELSE.to_string()),
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

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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

    #[test]
    fn test_to_oppgave_request() {
        let identitetsnummer = "12345678901".to_string();

        let request = create_oppgave_request(identitetsnummer.clone());

        assert_eq!(request.personident, Some(identitetsnummer));
        assert_eq!(request.oppgavetype, "KONT_BRUK");
        assert_eq!(request.tema, "GEN");
        assert_eq!(request.prioritet, PrioritetV1::Norm);
        assert!(request.orgnr.is_none());
        assert!(request.tildelt_enhetsnr.is_none());
    }
}
