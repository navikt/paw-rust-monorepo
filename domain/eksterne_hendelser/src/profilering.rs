use crate::vo::metadata::Metadata;
use crate::vo::profilert_til::ProfilertTil;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

pub const PAW_PROFILERING_TOPIC: &'static str = "paw.arbeidssoker-profilering-v1";

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profilering {
    #[serde_as(as = "DisplayFromStr")]
    pub id: Uuid,
    #[serde_as(as = "DisplayFromStr")]
    pub periode_id: Uuid,
    #[serde_as(as = "DisplayFromStr")]
    pub opplysninger_om_arbeidssoker_id: Uuid,
    pub sendt_inn_av: Metadata,
    pub profilert_til: ProfilertTil,
    pub jobbet_sammenhengende_seks_av_tolv_siste_mnd: bool,
    pub alder: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::{AvroDeserializer, AvroSerializer};
    use crate::vo::bruker::Bruker;
    use crate::vo::brukertype::BrukerType;
    use crate::vo::metadata::Metadata;
    use chrono::{DateTime, Utc};
    use mockito::Server;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_serde() {
        let mut mockito_server = Server::new_async().await;
        let guard = create_schema_registry_mock(&mut mockito_server)
            .await
            .unwrap();
        let schema_registry_settings = guard.schema_registry_settings;

        let serializer = AvroSerializer::new(schema_registry_settings.clone());
        let deserializer = AvroDeserializer::new(schema_registry_settings.clone());
        let value_naming_strategy =
            SubjectNameStrategy::TopicNameStrategy(PAW_PROFILERING_TOPIC.to_string(), false);

        let source_avro = create_dummy_profilering();

        let payload = serializer
            .serialize(&source_avro, &value_naming_strategy)
            .await
            .unwrap();
        let target_avro: Profilering = deserializer.deserialize(&payload).await.unwrap();

        assert_eq!(target_avro, source_avro);
    }

    fn create_dummy_profilering() -> Profilering {
        Profilering {
            id: Uuid::new_v4(),
            periode_id: Uuid::new_v4(),
            opplysninger_om_arbeidssoker_id: Uuid::new_v4(),
            sendt_inn_av: create_dummy_metadata(),
            profilert_til: ProfilertTil::AntattGodeMuligheter,
            jobbet_sammenhengende_seks_av_tolv_siste_mnd: false,
            alder: Some(42),
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
