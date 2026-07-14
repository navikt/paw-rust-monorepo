use crate::vo::annet::Annet;
use crate::vo::helse::Helse;
use crate::vo::jobbsituasjon::Jobbsituasjon;
use crate::vo::metadata::Metadata;
use crate::vo::utdanning::Utdanning;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

pub const PAW_OPPLYSNINGER_TOPIC: &'static str = "paw.opplysninger-om-arbeidssoeker-v1";

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Opplysninger {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Uuid,
    #[serde_as(as = "DisplayFromStr")]
    pub periode_id: Uuid,
    pub sendt_inn_av: Metadata,
    pub utdanning: Option<Utdanning>,
    pub helse: Option<Helse>,
    pub jobbsituasjon: Jobbsituasjon,
    pub annet: Option<Annet>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::{AvroDeserializer, AvroSerializer};
    use crate::vo::bruker::Bruker;
    use crate::vo::brukertype::BrukerType;
    use crate::vo::ja_nei_vet_ikke::JaNeiVetIkke;
    use crate::vo::jobbsituasjon::{Beskrivelse, BeskrivelseMedDetaljer};
    use crate::vo::metadata::Metadata;
    use chrono::{DateTime, Utc};
    use mockito::Server;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use std::collections::HashMap;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_serde() {
        let mut mockito_server = Server::new_async().await;
        let schema_registry_settings = create_schema_registry_mock(&mut mockito_server)
            .await
            .unwrap();

        let serializer = AvroSerializer::new(schema_registry_settings.clone());
        let deserializer = AvroDeserializer::new(schema_registry_settings.clone());
        let value_naming_strategy =
            SubjectNameStrategy::TopicNameStrategy(PAW_OPPLYSNINGER_TOPIC.to_string(), false);

        let source_avro = create_dummy_opplysninger();

        let payload = serializer
            .serialize(&source_avro, &value_naming_strategy)
            .await
            .unwrap();
        let target_avro: Opplysninger = deserializer.deserialize(&payload).await.unwrap();

        assert_eq!(target_avro, source_avro);
    }

    fn create_dummy_opplysninger() -> Opplysninger {
        Opplysninger {
            id: Uuid::new_v4(),
            periode_id: Uuid::new_v4(),
            sendt_inn_av: create_dummy_metadata(),
            utdanning: Some(Utdanning {
                nus: "1234".to_string(),
                bestaatt: Some(JaNeiVetIkke::Ja),
                godkjent: Some(JaNeiVetIkke::Ja),
            }),
            helse: Some(Helse {
                helsetilstand_hindrer_arbeid: JaNeiVetIkke::Nei,
            }),
            jobbsituasjon: Jobbsituasjon {
                beskrivelser: vec![BeskrivelseMedDetaljer {
                    beskrivelse: Beskrivelse::HarBlittSagtOpp,
                    detaljer: HashMap::from([
                        ("oppsigelsesdato".to_string(), "2024-06-30".to_string()),
                        ("arbeidsgiver".to_string(), "Test AS".to_string()),
                    ]),
                }],
            },
            annet: Some(Annet {
                andre_forhold_hindrer_arbeid: Some(JaNeiVetIkke::Nei),
            }),
        }
    }

    fn create_dummy_metadata() -> Metadata {
        Metadata {
            tidspunkt: datetime_rfc3339("2026-06-30T12:00:00Z"),
            utfoert_av: Bruker {
                bruker_type: BrukerType::Sluttbruker,
                id: "01017012345".to_string(),
                sikkerhetsnivaa: Some("tokenx:Level4".to_string()),
            },
            kilde: "test-system".to_string(),
            aarsak: "Test".to_string(),
            tidspunkt_fra_kilde: None,
        }
    }

    fn datetime_rfc3339(input: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(input)
            .unwrap_or_else(|e| panic!("Ugyldig RFC 3339-tidspunkt '{input}': {e}"))
            .with_timezone(&Utc)
    }
}
