use crate::logic::process::PayloadProcessor;
use crate::model::dao::bekreftelse_paavegneav;
use crate::model::dao::bekreftelse_paavegneav::BekreftelsePaaVegneAvRow;
use crate::model::error::{DaoError, PayloadProcessorError};
use crate::model::result::ProcessorResult;
use eksterne_hendelser::bekreftelse::paa_vegne_av::{Handling, PaaVegneAv};
use paw_kafka::hwm::hwm_message_processor::ProcessorError;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use paw_avro_serde::serde::AvroDeserializer;

pub struct BekreftelsePaaVegneAvProcessor {
    pub deserializer: AvroDeserializer,
}

impl BekreftelsePaaVegneAvProcessor {
    pub fn new(schema_registry_settings: SrSettings) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_settings),
        }
    }
}

impl PayloadProcessor for BekreftelsePaaVegneAvProcessor {
    #[tracing::instrument(skip_all, fields(topic = %message.topic(), partition = %message.partition(), offset = %message.offset()))]
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<ProcessorResult, ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: PaaVegneAv = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;
                let handling = &hendelse.handling;
                let periode_id = hendelse.periode_id;
                let bekreftelsesloesning = hendelse.bekreftelsesloesning.as_ref().to_string();

                tracing::debug!("Mottok {}-hendelse", &hendelse);

                let rows = bekreftelse_paavegneav::select_by_periode_id(tx, &periode_id).await?;
                let count = rows.len();

                if count > 1 {
                    Err(DaoError::multiple_rows(message, "bekreftelse_paavegneav", count).into())
                } else if count == 1 {
                    let row = rows.first().expect("Ingen rad funnet");
                    match handling {
                        Handling::Start(_) => {
                            let mut bekreftelsesloesninger = row.bekreftelsesloesninger.clone();
                            bekreftelsesloesninger.push(bekreftelsesloesning);
                            bekreftelsesloesninger.sort_unstable();
                            bekreftelsesloesninger.dedup();
                            let bekreftelsesloesninger = bekreftelsesloesninger;
                            let updated_row =
                                BekreftelsePaaVegneAvRow::new(periode_id, bekreftelsesloesninger);
                            bekreftelse_paavegneav::update(tx, &updated_row).await?;
                        }
                        Handling::Stopp(_) => {
                            let bekreftelsesloesninger = row
                                .bekreftelsesloesninger
                                .iter()
                                .filter(|&l| l != &bekreftelsesloesning)
                                .map(|l| l.to_string())
                                .collect();
                            let updated_row =
                                BekreftelsePaaVegneAvRow::new(periode_id, bekreftelsesloesninger);
                            bekreftelse_paavegneav::update(tx, &updated_row).await?;
                        }
                    }

                    Ok(ProcessorResult::Continue)
                } else {
                    match hendelse.handling {
                        Handling::Start(_) => {
                            let bekreftelsesloesninger = vec![bekreftelsesloesning];
                            let row =
                                BekreftelsePaaVegneAvRow::new(periode_id, bekreftelsesloesninger);
                            bekreftelse_paavegneav::insert(tx, &row).await?;
                        }
                        Handling::Stopp(_) => {
                            tracing::warn!(
                                "Mottok stopp for på-vegne-av som ikke finnes i databasen"
                            );
                        }
                    }

                    Ok(ProcessorResult::Continue)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::PayloadProcessor;
    use crate::logic::process::bekreftelse_paavegneav_process::BekreftelsePaaVegneAvProcessor;
    use crate::model::dao::bekreftelse_paavegneav;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use mockito::{Mock, Server, ServerGuard};
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_mock::schema_definitions::avro_schemas;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock_2;
    use sqlx::{PgPool, Postgres, Transaction};
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_start_paavegneav, create_dummy_stopp_paavegneav,
    };
    use uuid::Uuid;

    const BEKREFTELSE_PAAVEGNEAV_TOPIC: &str = "paw.arbeidssoker-bekreftelse-paavegneav-v1";

    #[tokio::test]
    async fn test_process_messages() {
        let context = TestContext::new().await;

        test_process_paavegneav_start_1(&context).await;
        test_process_paavegneav_start_2(&context).await;
        test_process_paavegneav_stopp(&context).await;
    }

    async fn test_process_paavegneav_start_1(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id;
        let periode_id = context.periode_id;

        let key = arbeidssoeker_id.to_be_bytes().to_vec();
        let payload = create_dummy_start_paavegneav(
            periode_id,
            Bekreftelsesloesning::Arbeidssoekerregisteret,
        );
        let message = context
            .avro_generator
            .create_avro_message(BEKREFTELSE_PAAVEGNEAV_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx, &message).await;
        assert!(result_1.is_ok());
        let paavegneav_rows_1 = bekreftelse_paavegneav::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(paavegneav_rows_1.len(), 1);
        let paavegneav_row_1 = paavegneav_rows_1
            .get(0)
            .expect("Ingen bekreftelse på-vegne-av funnet");
        assert_eq!(paavegneav_row_1.periode_id, periode_id);
        assert_eq!(
            paavegneav_row_1.bekreftelsesloesninger,
            vec![
                Bekreftelsesloesning::Arbeidssoekerregisteret
                    .as_ref()
                    .to_string()
            ]
        );
    }

    async fn test_process_paavegneav_start_2(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id;
        let periode_id = context.periode_id;

        let key = arbeidssoeker_id.to_be_bytes().to_vec();
        let payload = create_dummy_start_paavegneav(
            periode_id,
            Bekreftelsesloesning::FriskmeldtTilArbeidsformidling,
        );
        let message = context
            .avro_generator
            .create_avro_message(BEKREFTELSE_PAAVEGNEAV_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx, &message).await;
        assert!(result_1.is_ok());
        let paavegneav_rows_1 = bekreftelse_paavegneav::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(paavegneav_rows_1.len(), 1);
        let paavegneav_row_1 = paavegneav_rows_1
            .get(0)
            .expect("Ingen bekreftelse på-vegne-av funnet");
        assert_eq!(paavegneav_row_1.periode_id, periode_id);
        assert_eq!(
            paavegneav_row_1.bekreftelsesloesninger,
            vec![
                Bekreftelsesloesning::Arbeidssoekerregisteret
                    .as_ref()
                    .to_string(),
                Bekreftelsesloesning::FriskmeldtTilArbeidsformidling
                    .as_ref()
                    .to_string()
            ]
        );
    }

    async fn test_process_paavegneav_stopp(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id;
        let periode_id = context.periode_id;
        
        let key = arbeidssoeker_id.to_be_bytes().to_vec();
        let payload = create_dummy_stopp_paavegneav(
            periode_id,
            Bekreftelsesloesning::Arbeidssoekerregisteret,
        );
        let message = context
            .avro_generator
            .create_avro_message(BEKREFTELSE_PAAVEGNEAV_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx, &message).await;
        assert!(result_1.is_ok());
        let paavegneav_rows_1 = bekreftelse_paavegneav::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(paavegneav_rows_1.len(), 1);
        let paavegneav_row_1 = paavegneav_rows_1
            .get(0)
            .expect("Ingen bekreftelse på-vegne-av funnet");
        assert_eq!(paavegneav_row_1.periode_id, periode_id);
        assert_eq!(
            paavegneav_row_1.bekreftelsesloesninger,
            vec![
                Bekreftelsesloesning::FriskmeldtTilArbeidsformidling
                    .as_ref()
                    .to_string()
            ]
        );
    }

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        avro_generator: AvroGenerator,
        processor: BekreftelsePaaVegneAvProcessor,
        arbeidssoeker_id: i64,
        periode_id: Uuid,
    }

    impl TestContext {

        async fn new() -> Self {
            let pdl_mock_responses = default_pdl_mock_responses();
            let mut mockito_server = Server::new_async().await;
            let schemas = avro_schemas();
            let schema_registry_guard = create_schema_registry_mock_2(&mut mockito_server, schemas)
                    .await
                    .expect("Failed to create schema registry mock");
            let schema_registry_settings = schema_registry_guard.schema_registry_settings;

            let pdl_mock_guard = init_pdl_mock(&mut mockito_server, pdl_mock_responses)
                    .await
                    .expect("Kunne ikke initialisere PDL mock server");

            let mut schema_registry_mocks = schema_registry_guard.mocks;
            let mut mocks = pdl_mock_guard.mocks;
            mocks.append(&mut schema_registry_mocks);

            let postgres_guard = setup_postgres_container()
                    .await
                    .expect("Failed to start Postgres container");
            sqlx::migrate!("./migrations")
                    .run(&postgres_guard.pg_pool)
                    .await
                    .expect("Failed to run migrations");

            Self {
                mockito_server,
                mocks,
                pg_pool: postgres_guard.pg_pool,
                avro_generator: AvroGenerator::new(schema_registry_settings.clone()),
                processor: BekreftelsePaaVegneAvProcessor::new(schema_registry_settings.clone()),
                arbeidssoeker_id: 12345,
                periode_id: Uuid::new_v4(),
            }
        }

        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }
    }
}
