use crate::vo::Deltakelse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiltakshistorikkV1Request {
    pub identer: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiltakshistorikkV1Response {
    pub historikk: Vec<Deltakelse>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vo::ArenaStatus;

    #[test]
    fn request_uses_camel_case_and_response_deserializes_all_participation_types() {
        let request = TiltakshistorikkV1Request {
            identer: vec!["01017012345".to_string()],
        };
        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({"identer": ["01017012345"]})
        );

        let response: TiltakshistorikkV1Response = serde_json::from_value(serde_json::json!({
            "historikk": [
                {
                    "type": "ArenaDeltakelse",
                    "norskIdent": "01017012345",
                    "startDato": null,
                    "sluttDato": null,
                    "opprettetTidspunkt": "2026-01-01T00:00:00Z",
                    "oppdatertTidspunkt": "2026-01-02T00:00:00Z",
                    "id": "arena-id",
                    "tittel": "Arena-tiltak",
                    "arenaId": 123,
                    "status": "GJENNOMFORES",
                    "tiltakstype": {"tiltakskode": "CODE", "navn": "Tiltak"},
                    "gjennomforing": {"id": "g1", "navn": null, "deltidsprosent": null},
                    "arrangor": {
                        "hovedenhet": null,
                        "underenhet": {"organisasjonsnummer": "123456789", "navn": "Arrangør"}
                    },
                    "deltidsprosent": null,
                    "dagerPerUke": null,
                    "opphav": "ARENA"
                },
                {
                    "type": "TeamKometDeltakelse",
                    "norskIdent": "01017012345",
                    "startDato": "2026-01-01",
                    "sluttDato": null,
                    "opprettetTidspunkt": "2026-01-01T00:00:00Z",
                    "oppdatertTidspunkt": "2026-01-02T00:00:00Z",
                    "id": "komet-id",
                    "tittel": "Komet-tiltak",
                    "status": {
                        "type": "DELTAR",
                        "aarsak": null,
                        "opprettetTidspunkt": "2026-01-01T00:00:00Z"
                    },
                    "tiltakstype": {"tiltakskode": "OPPFOLGING", "navn": "Oppfølging"},
                    "gjennomforing": {"id": "g2", "navn": "Gjennomføring", "deltidsprosent": 50.0},
                    "arrangor": {
                        "hovedenhet": {
                            "organisasjonsnummer": "123456789",
                            "navn": "Hovedenhet"
                        },
                        "underenhet": {"organisasjonsnummer": "987654321", "navn": null}
                    },
                    "deltidsprosent": 50.0,
                    "dagerPerUke": 3.0,
                    "opphav": "TEAM_KOMET"
                },
                {
                    "type": "TeamTiltakAvtale",
                    "norskIdent": "01017012345",
                    "startDato": null,
                    "sluttDato": null,
                    "opprettetTidspunkt": "2026-01-01T00:00:00Z",
                    "oppdatertTidspunkt": "2026-01-02T00:00:00Z",
                    "id": "tiltak-id",
                    "tittel": "Arbeidstrening",
                    "tiltakstype": {"tiltakskode": "ARBEIDSTRENING", "navn": "Arbeidstrening"},
                    "status": "GJENNOMFORES",
                    "stillingsprosent": 100.0,
                    "dagerPerUke": 5.0,
                    "arbeidsgiver": {"organisasjonsnummer": "123456789", "navn": "Arbeidsgiver"},
                    "opphav": "TEAM_TILTAK"
                }
            ]
        }))
        .unwrap();

        assert_eq!(response.historikk.len(), 3);
        assert!(matches!(
            response.historikk[0],
            Deltakelse::ArenaDeltakelse(_)
        ));
        assert!(matches!(
            response.historikk[1],
            Deltakelse::TeamKometDeltakelse(_)
        ));
        assert!(matches!(
            response.historikk[2],
            Deltakelse::TeamTiltakAvtale(_)
        ));

        let serialized = serde_json::to_value(&response).unwrap();
        assert_eq!(serialized["historikk"][0]["type"], "ArenaDeltakelse");
        assert_eq!(serialized["historikk"][1]["type"], "TeamKometDeltakelse");
        assert_eq!(serialized["historikk"][2]["type"], "TeamTiltakAvtale");
        let round_trip: TiltakshistorikkV1Response = serde_json::from_value(serialized).unwrap();
        assert_eq!(round_trip, response);
    }

    #[test]
    fn unknown_enum_values_round_trip_without_loss() {
        let status: ArenaStatus = serde_json::from_str(r#""NEW_STATUS""#).unwrap();
        assert_eq!(status, ArenaStatus::Ukjent("NEW_STATUS".to_string()));
        assert_eq!(serde_json::to_string(&status).unwrap(), r#""NEW_STATUS""#);
    }
}
