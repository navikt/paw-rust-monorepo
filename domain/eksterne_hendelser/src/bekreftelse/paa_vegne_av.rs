use crate::bekreftelse::bekreftelsesloesning::Bekreftelsesloesning;
use crate::bekreftelse::start::Start;
use crate::bekreftelse::stopp::Stopp;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use uuid::Uuid;

pub const BEKREFTELSE_PAAVEGNEAV_TOPIC: &'static str = "paw.arbeidssoker-bekreftelse-paavegneav-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Handling {
    Start(Start),
    Stopp(Stopp),
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaaVegneAv {
    #[serde_as(as = "DisplayFromStr")]
    pub periode_id: Uuid,
    pub bekreftelsesloesning: Bekreftelsesloesning,
    pub handling: Handling,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::{AvroDeserializer, AvroSerializer};
    use mockito::Server;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_serde() {
        let mut mockito_server = Server::new_async().await;
        let schema_registry_settings = create_schema_registry_mock(&mut mockito_server)
            .await
            .unwrap();

        let serializer = AvroSerializer::new(schema_registry_settings.clone());
        let deserializer = AvroDeserializer::new(schema_registry_settings.clone());
        let strategy =
            SubjectNameStrategy::TopicNameStrategy(BEKREFTELSE_PAAVEGNEAV_TOPIC.to_string(), false);
        let source_start = create_dummy_paavegneav_start();
        let source_stopp = create_dummy_paavegneav_stopp();

        let payload_start = serializer
            .serialize(&source_start, &strategy)
            .await
            .unwrap();
        let payload_stopp = serializer
            .serialize(&source_stopp, &strategy)
            .await
            .unwrap();

        let target_start: PaaVegneAv = deserializer.deserialize(&payload_start).await.unwrap();
        let target_stopp: PaaVegneAv = deserializer.deserialize(&payload_stopp).await.unwrap();

        assert_eq!(source_start, target_start);
        assert_eq!(source_stopp, target_stopp);
    }

    fn create_dummy_paavegneav(handling: Handling) -> PaaVegneAv {
        PaaVegneAv {
            periode_id: Uuid::new_v4(),
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
            handling,
        }
    }

    fn create_dummy_paavegneav_start() -> PaaVegneAv {
        create_dummy_paavegneav(Handling::Start(Start {
            interval_ms: 5,
            grace_ms: 3,
        }))
    }

    fn create_dummy_paavegneav_stopp() -> PaaVegneAv {
        create_dummy_paavegneav(Handling::Stopp(Stopp { frist_brutt: true }))
    }
}
