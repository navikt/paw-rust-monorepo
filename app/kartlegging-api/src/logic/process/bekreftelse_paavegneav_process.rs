use crate::logic::mutation::bekreftelse_paavegneav_mutation;
use eksterne_hendelser::bekreftelse::paa_vegne_av::PaaVegneAv;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};

pub struct BekreftelsePaaVegneAvProcessor {
    pub deserializer: AvroDeserializer,
}

impl BekreftelsePaaVegneAvProcessor {
    pub fn new(schema_registry_settings: SrSettings) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_settings),
        }
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: PaaVegneAv = self.deserializer.deserialize(payload).await.map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;

        tracing::info!("Mottok hendelse: {:?}", &hendelse);

        bekreftelse_paavegneav_mutation::lagre_hendelse(tx, &hendelse).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::bekreftelse_paavegneav_process::BekreftelsePaaVegneAvProcessor;
    use crate::model::dao::bekreftelse_paavegneav;
    use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use eksterne_hendelser::serde::AvroSerializer;
    use mockito::{Mock, Server, ServerGuard};
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use serde::Serialize;
    use sqlx::{PgPool, Postgres, Transaction};
    use test_data_generator::eksterne_hendelser::{
        create_dummy_paavegneav_start, create_dummy_paavegneav_stopp,
    };
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        avro_serializer: AvroSerializer,
        processor: BekreftelsePaaVegneAvProcessor,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }

        async fn create_avro_payload(
            &self,
            topic: &'static str,
            payload: impl Serialize,
        ) -> Vec<u8> {
            let strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), false);
            self.avro_serializer
                .serialize(payload, &strategy)
                .await
                .expect("Kunne ikke serialisere melding")
        }
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let pdl_mock_responses = default_pdl_mock_responses();
            let mut mockito_server = Server::new_async().await;

            let schema_registry_guard = create_schema_registry_mock(&mut mockito_server)
                .await
                .expect("Failed to create schema registry mock");
            let schema_registry_settings = schema_registry_guard.schema_registry_settings;

            let pdl_mock_guard = init_pdl_mock(&mut mockito_server, pdl_mock_responses)
                .await
                .expect("Kunne ikke initialisere PDL mock server");

            let mut schema_registry_mocks = schema_registry_guard.mocks;
            let mut mocks = pdl_mock_guard.mocks;
            mocks.append(&mut schema_registry_mocks);

            let postgres_guard = setup_postgres_container(5432)
                .await
                .expect("Failed to start Postgres container");
            sqlx::migrate!("./migrations")
                .run(&postgres_guard.pg_pool)
                .await
                .expect("Failed to run migrations");

            TestContext {
                mockito_server,
                mocks,
                pg_pool: postgres_guard.pg_pool,
                avro_serializer: AvroSerializer::new(schema_registry_settings.clone()),
                processor: BekreftelsePaaVegneAvProcessor::new(schema_registry_settings.clone()),
            }
        })
        .await
    }

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        let periode_id = Uuid::new_v4();
        let paavegneav_1 = create_dummy_paavegneav_start(
            periode_id,
            Bekreftelsesloesning::Arbeidssoekerregisteret,
        );
        let payload_1 = context
            .create_avro_payload(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav_1.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx, &payload_1).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let paavegneav_2 = create_dummy_paavegneav_start(
            periode_id,
            Bekreftelsesloesning::FriskmeldtTilArbeidsformidling,
        );
        let payload_2 = context
            .create_avro_payload(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav_2.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result_2 = context.processor.process_payload(&mut tx, &payload_2).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");
        assert!(result_2.is_ok());

        let mut tx = context.start_tx().await;
        let rows_1 = bekreftelse_paavegneav::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(rows_1.len(), 1);
        let row_1 = rows_1.get(0).expect("Ingen bekreftelse på-vegne-av funnet");
        assert_eq!(row_1.periode_id, periode_id);
        assert_eq!(
            row_1.bekreftelsesloesninger,
            vec![
                Bekreftelsesloesning::Arbeidssoekerregisteret
                    .as_ref()
                    .to_string(),
                Bekreftelsesloesning::FriskmeldtTilArbeidsformidling
                    .as_ref()
                    .to_string()
            ]
        );

        let paavegneav_3 = create_dummy_paavegneav_stopp(
            periode_id,
            Bekreftelsesloesning::Arbeidssoekerregisteret,
        );
        let payload_3 = context
            .create_avro_payload(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav_3.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result_3 = context.processor.process_payload(&mut tx, &payload_3).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");
        assert!(result_3.is_ok());

        let mut tx = context.start_tx().await;
        let rows_2 = bekreftelse_paavegneav::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente bekreftelse på-vegne-av");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(rows_2.len(), 1);
        let row = rows_2.get(0).expect("Ingen bekreftelse på-vegne-av funnet");
        assert_eq!(row.periode_id, periode_id);
        assert_eq!(
            row.bekreftelsesloesninger,
            vec![
                Bekreftelsesloesning::FriskmeldtTilArbeidsformidling
                    .as_ref()
                    .to_string()
            ]
        );
    }
}
