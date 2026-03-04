use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Deserializer, Serialize};
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
    #[serde(deserialize_with = "deserialize_jackson_datetime")]
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
    #[serde(deserialize_with = "deserialize_jackson_date")]
    pub aktiv: NaiveDate,
    #[serde(default, deserialize_with = "deserialize_jackson_date_option")]
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

/// Jackson serialiserer LocalDateTime som array: [år, måned, dag, time, minutt, sekund, nano]
fn deserialize_jackson_datetime<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
where D: Deserializer<'de> {
    let arr: Vec<i64> = Vec::deserialize(deserializer)?;
    let (aar, mnd, dag, time, min, sek, nano) = match arr.as_slice() {
        [aar, mnd, dag, time, min, sek, nano] => (*aar, *mnd, *dag, *time, *min, *sek, *nano),
        [aar, mnd, dag, time, min, sek] => (*aar, *mnd, *dag, *time, *min, *sek, 0),
        [aar, mnd, dag, time, min] => (*aar, *mnd, *dag, *time, *min, 0, 0),
        _ => return Err(serde::de::Error::custom(
            format!("Forventet 5-7 elementer i datetime-array, fikk {}", arr.len())
        )),
    };
    NaiveDate::from_ymd_opt(aar as i32, mnd as u32, dag as u32)
        .and_then(|d| d.and_hms_nano_opt(time as u32, min as u32, sek as u32, nano as u32))
        .ok_or_else(|| serde::de::Error::custom(format!("Ugyldig datetime-array: {:?}", arr)))
}

/// Jackson serialiserer LocalDate som array: [år, måned, dag]
fn deserialize_jackson_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where D: Deserializer<'de> {
    let arr: Vec<i32> = Vec::deserialize(deserializer)?;
    let [aar, mnd, dag] = arr.as_slice() else {
        return Err(serde::de::Error::custom(
            format!("Forventet 3 elementer i date-array, fikk {}", arr.len())
        ));
    };
    NaiveDate::from_ymd_opt(*aar, *mnd as u32, *dag as u32)
        .ok_or_else(|| serde::de::Error::custom(format!("Ugyldig date-array: {:?}", arr)))
}

fn deserialize_jackson_date_option<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
where D: Deserializer<'de> {
    let arr: Option<Vec<i32>> = Option::deserialize(deserializer)?;
    arr.map(|arr| {
        let [aar, mnd, dag] = arr.as_slice() else {
            return Err(serde::de::Error::custom(
                format!("Forventet 3 elementer i date-array, fikk {}", arr.len())
            ));
        };
        NaiveDate::from_ymd_opt(*aar, *mnd as u32, *dag as u32)
            .ok_or_else(|| serde::de::Error::custom(format!("Ugyldig date-array: {:?}", arr)))
    }).transpose()
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
                "tidspunkt": [2023, 2, 23, 8, 58, 23, 832000000]
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
                    "aktiv": [2023, 2, 23],
                    "frist": [2023, 2, 23]
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
                "tidspunkt": [2023, 3, 1, 12, 0]
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
        assert_eq!(melding.hendelse.tidspunkt, NaiveDate::from_ymd_opt(2023, 3, 1).unwrap().and_hms_opt(12, 0, 0).unwrap());
        assert_eq!(melding.utfort_av, None);
        assert_eq!(melding.oppgave.oppgave_id, 99999);
        assert_eq!(melding.oppgave.tilordning, None);
        assert_eq!(melding.oppgave.kategorisering, None);
        assert_eq!(melding.oppgave.behandlingsperiode, None);
        assert_eq!(melding.oppgave.bruker, None);
    }

    #[test]
    fn test_deserialize_jackson_datetime_array_lengder() {
        let json_5 = r#"{"hendelsestype":"OPPGAVE_OPPRETTET","tidspunkt":[2023,3,1,12,0]}"#;
        let h5: OppgaveHendelse = serde_json::from_str(json_5).unwrap();
        assert_eq!(h5.tidspunkt, NaiveDate::from_ymd_opt(2023, 3, 1).unwrap().and_hms_opt(12, 0, 0).unwrap());

        let json_6 = r#"{"hendelsestype":"OPPGAVE_OPPRETTET","tidspunkt":[2023,3,1,12,30,45]}"#;
        let h6: OppgaveHendelse = serde_json::from_str(json_6).unwrap();
        assert_eq!(h6.tidspunkt, NaiveDate::from_ymd_opt(2023, 3, 1).unwrap().and_hms_opt(12, 30, 45).unwrap());

        let json_7 = r#"{"hendelsestype":"OPPGAVE_OPPRETTET","tidspunkt":[2023,3,1,12,30,45,500000000]}"#;
        let h7: OppgaveHendelse = serde_json::from_str(json_7).unwrap();
        assert_eq!(h7.tidspunkt, NaiveDate::from_ymd_opt(2023, 3, 1).unwrap().and_hms_nano_opt(12, 30, 45, 500000000).unwrap());
    }
}
