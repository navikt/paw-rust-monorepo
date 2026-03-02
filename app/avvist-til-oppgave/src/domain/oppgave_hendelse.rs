use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveHendelseMelding {
    pub hendelse: OppgaveHendelse,
    pub utfort_av: Option<OppgaveUtfortAv>,
    pub oppgave: EksternOppgave,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveHendelse {
    pub hendelsestype: OppgaveHendelsetype,
    pub tidspunkt: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveUtfortAv {
    pub nav_ident: Option<String>,
    pub enhetsnr: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EksternOppgave {
    pub oppgave_id: i64,
    pub versjon: i32,
    pub tilordning: Option<OppgaveTilordning>,
    pub kategorisering: Option<OppgaveKategorisering>,
    pub behandlingsperiode: Option<OppgaveBehandlingsperiode>,
    pub bruker: Option<OppgaveBruker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveTilordning {
    pub enhetsnr: Option<String>,
    pub enhetsmappe_id: Option<i64>,
    pub nav_ident: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveKategorisering {
    pub tema: String,
    pub oppgavetype: String,
    pub behandlingstema: Option<String>,
    pub behandlingstype: Option<String>,
    pub prioritet: OppgavePrioritet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OppgaveBehandlingsperiode {
    pub aktiv: NaiveDate,
    pub frist: Option<NaiveDate>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_oppgave_hendelse_melding() {
        let json = r#"
        {
            "hendelse": {
                "hendelsestype": "OPPGAVE_OPPRETTET",
                "tidspunkt": "2023-02-23T08:58:23.832"
            },
            "utfortAv": {
                "navIdent": "Z991459",
                "enhetsnr": "2990"
            },
            "oppgave": {
                "oppgaveId": 125046,
                "versjon": 1,
                "tilordning": {
                    "enhetsnr": "2990",
                    "enhetsmappeId": null,
                    "navIdent": "Z991459"
                },
                "kategorisering": {
                    "tema": "SYK",
                    "oppgavetype": "BEH_HENV",
                    "behandlingstema": null,
                    "behandlingstype": "ae0160",
                    "prioritet": "NORMAL"
                },
                "behandlingsperiode": {
                    "aktiv": "2023-02-23",
                    "frist": "2023-02-23"
                },
                "bruker": {
                    "ident": "12345678901",
                    "identType": "FOLKEREGISTERIDENT"
                }
            }
        }
        "#;

        let melding: OppgaveHendelseMelding = serde_json::from_str(json).unwrap();

        assert_eq!(melding.hendelse.hendelsestype, OppgaveHendelsetype::OppgaveOpprettet);
        assert_eq!(melding.hendelse.tidspunkt, NaiveDateTime::parse_from_str("2023-02-23T08:58:23.832", "%Y-%m-%dT%H:%M:%S%.f").unwrap());

        let utfort_av = melding.utfort_av.unwrap();
        assert_eq!(utfort_av.nav_ident, Some("Z991459".to_string()));
        assert_eq!(utfort_av.enhetsnr, Some("2990".to_string()));

        let oppgave = melding.oppgave;
        assert_eq!(oppgave.oppgave_id, 125046);
        assert_eq!(oppgave.versjon, 1);

        let tilordning = oppgave.tilordning.unwrap();
        assert_eq!(tilordning.enhetsnr, Some("2990".to_string()));
        assert_eq!(tilordning.enhetsmappe_id, None);
        assert_eq!(tilordning.nav_ident, Some("Z991459".to_string()));

        let kategorisering = oppgave.kategorisering.unwrap();
        assert_eq!(kategorisering.tema, "SYK");
        assert_eq!(kategorisering.oppgavetype, "BEH_HENV");
        assert_eq!(kategorisering.behandlingstema, None);
        assert_eq!(kategorisering.behandlingstype, Some("ae0160".to_string()));
        assert_eq!(kategorisering.prioritet, OppgavePrioritet::Normal);

        let behandlingsperiode = oppgave.behandlingsperiode.unwrap();
        assert_eq!(behandlingsperiode.aktiv, NaiveDate::from_ymd_opt(2023, 2, 23).unwrap());
        assert_eq!(behandlingsperiode.frist, Some(NaiveDate::from_ymd_opt(2023, 2, 23).unwrap()));

        let bruker = oppgave.bruker.unwrap();
        assert_eq!(bruker.ident, "12345678901");
        assert_eq!(bruker.ident_type, OppgaveIdentType::FolkeregisterIdent);
    }

    #[test]
    fn test_deserialize_med_nullable_felter() {
        let json = r#"
        {
            "hendelse": {
                "hendelsestype": "OPPGAVE_FERDIGSTILT",
                "tidspunkt": "2023-03-01T12:00:00"
            },
            "utfortAv": null,
            "oppgave": {
                "oppgaveId": 99999,
                "versjon": 3,
                "tilordning": null,
                "kategorisering": null,
                "behandlingsperiode": null,
                "bruker": null
            }
        }
        "#;

        let melding: OppgaveHendelseMelding = serde_json::from_str(json).unwrap();

        assert_eq!(melding.hendelse.hendelsestype, OppgaveHendelsetype::OppgaveFerdigstilt);
        assert_eq!(melding.utfort_av, None);
        assert_eq!(melding.oppgave.oppgave_id, 99999);
        assert_eq!(melding.oppgave.tilordning, None);
        assert_eq!(melding.oppgave.kategorisering, None);
        assert_eq!(melding.oppgave.behandlingsperiode, None);
        assert_eq!(melding.oppgave.bruker, None);
    }
}
