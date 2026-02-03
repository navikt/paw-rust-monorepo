use crate::kafka::kafka_message::KafkaMessage;

pub fn deserialize_value_to_string(message: KafkaMessage) -> Result<String, Box<dyn std::error::Error>> {
    let value_bytes = message.payload;
    let value_str =
        String::from_utf8(value_bytes).map_err(|_e| format!("Invalid UTF-8 in payload"))?;
    Ok(value_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kan_deserialisere_verdi() {
        let avvist_hendelse = avvist_hendelse_json();
        let kafka_message = KafkaMessage {
            topic: "test_topic".to_string(),
            partition: 0,
            offset: 0,
            headers: None,
            key: vec![],
            payload: avvist_hendelse_json().as_bytes().to_vec(),
            timestamp: chrono::Utc::now(),
        };
        let deserialisert_verdi = deserialize_value_to_string(kafka_message).unwrap();
        assert_eq!(deserialisert_verdi, avvist_hendelse);
    }

    // language=JSON
    fn avvist_hendelse_json() -> &'static str {
        r#"
        {
          "endret": false,
          "hendelse": {
            "hendelseId": "a0f00264-90e2-4747-8cdd-8d4a8c75e5b0",
            "hendelseType": "intern.v1.avvist",
            "metadata": {
              "tidspunkt": "2026-02-03T09:43:36.928144200Z",
              "utfoertAv": {
                "type": "SLUTTBRUKER",
                "id": "28521086957"
              },
              "kilde": "europe-north1-docker.pkg.dev/nais-management-233d/paw/paw-arbeidssokerregisteret-api-inngang:26.01.20.407-1",
              "aarsak": "Er under 18 år"
            },
            "kafkaOffset": 4818,
            "data": {
              "hendelseId": "a0f00264-90e2-4747-8cdd-8d4a8c75e5b0",
              "id": 144850,
              "identitetsnummer": "28521086957",
              "metadata": {
                "tidspunkt": "2026-02-03T09:43:36.928144200Z",
                "utfoertAv": {
                  "type": "SLUTTBRUKER",
                  "id": "28521086957",
                  "sikkerhetsnivaa": "tokenx:Level4"
                },
                "kilde": "europe-north1-docker.pkg.dev/nais-management-233d/paw/paw-arbeidssokerregisteret-api-inngang:26.01.20.407-1",
                "aarsak": "Er under 18 år"
              },
              "opplysninger": [
                "HAR_NORSK_ADRESSE",
                "ER_UNDER_18_AAR",
                "IKKE_ANSATT",
                "INGEN_INFORMASJON_OM_OPPHOLDSTILLATELSE",
                "ER_EU_EOES_STATSBORGER",
                "INGEN_FLYTTE_INFORMASJON",
                "ER_NORSK_STATSBORGER",
                "SAMME_SOM_INNLOGGET_BRUKER",
                "IKKE_SYSTEM",
                "HAR_REGISTRERT_ADRESSE_I_EU_EOES",
                "BOSATT_ETTER_FREG_LOVEN"
              ],
              "handling": "/api/v2/arbeidssoker/kanStartePeriode",
              "hendelseType": "intern.v1.avvist"
            },
            "kafkaPartition": 5,
            "merged": false,
            "traceparent": "00-8c8f154a7a896a42f68af8d32967d4de-0609ab2a7f698515-01"
        }
}
    "#
    }
}