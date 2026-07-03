use crate::vo::metadata::Metadata;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

pub const PERIODE_TOPIC: &'static str = "paw.arbeidssokerperioder-v1";

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Periode {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Uuid,
    pub identitetsnummer: String,
    pub startet: Metadata,
    pub avsluttet: Option<Metadata>,
}

impl Periode {
    pub fn is_active(&self) -> bool {
        self.avsluttet.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vo::bruker::Bruker;
    use crate::vo::brukertype::BrukerType;
    use crate::vo::metadata::Metadata;
    use crate::serde::{AvroDeserializer, AvroSerializer};
    use chrono::{DateTime, Utc};
    use mockito::Server;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;

    #[test]
    fn test_periode_is_active() {
        let startet_periode = create_dummy_startet_periode();
        let avsluttet_periode = create_dummy_avsluttet_periode();

        assert!(startet_periode.is_active());
        assert!(!avsluttet_periode.is_active());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_serde() {
        let mut mockito_server = Server::new_async().await;
        let schema_registry_settings = create_schema_registry_mock(&mut mockito_server)
            .await
            .unwrap();

        let serializer = AvroSerializer::new(schema_registry_settings.clone());
        let deserializer = AvroDeserializer::new(schema_registry_settings.clone());
        let value_naming_strategy =
            SubjectNameStrategy::TopicNameStrategy(PERIODE_TOPIC.to_string(), false);

        let source_avro = create_dummy_startet_periode();

        let payload = serializer
            .serialize(&source_avro, &value_naming_strategy)
            .await
            .unwrap();
        let target_avro: Periode = deserializer.deserialize(&payload).await.unwrap();

        assert_eq!(target_avro, source_avro);
    }

    fn create_dummy_startet_periode() -> Periode {
        Periode {
            id: Uuid::new_v4(),
            identitetsnummer: "01017012345".to_string(),
            startet: create_dummy_metadata(),
            avsluttet: None,
        }
    }

    fn create_dummy_avsluttet_periode() -> Periode {
        Periode {
            id: Uuid::new_v4(),
            identitetsnummer: "01017012345".to_string(),
            startet: create_dummy_metadata(),
            avsluttet: Some(create_dummy_metadata()),
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
