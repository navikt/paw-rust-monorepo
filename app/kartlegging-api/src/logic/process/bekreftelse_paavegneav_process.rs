use crate::logic::process::PayloadProcessor;
use crate::model::dao::bekreftelse_paavegneav;
use crate::model::dao::bekreftelse_paavegneav::BekreftelsePaaVegneAvRow;
use crate::model::error::{DaoError, PayloadProcessorError};
use eksterne_hendelser::bekreftelse::paa_vegne_av::{Handling, PaaVegneAv};
use eksterne_hendelser::serde::AvroDeserializer;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
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
}

impl PayloadProcessor for BekreftelsePaaVegneAvProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<(), ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: PaaVegneAv = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok PaaVegneAv-hendelse");

                let handling = &hendelse.handling;
                let periode_id = hendelse.periode_id;
                let bekreftelsesloesning = hendelse.bekreftelsesloesning.as_ref().to_string();
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

                    Ok(())
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

                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::bekreftelse_paavegneav_process::BekreftelsePaaVegneAvProcessor;
    use crate::logic::process::PayloadProcessor;
    use crate::model::dao::bekreftelse_paavegneav;
    use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use mockito::{Mock, Server, ServerGuard};
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use sqlx::{PgPool, Postgres, Transaction};
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_paavegneav_start, create_dummy_paavegneav_stopp,
    };
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        test_process_paavegneav_start_1(context).await;
        test_process_paavegneav_start_2(context).await;
        test_process_paavegneav_stopp(context).await;
    }

    async fn test_process_paavegneav_start_1(context: &TestContext) {
        let periode_id = context.periode_id;

        let paavegneav = create_dummy_paavegneav_start(
            periode_id,
            Bekreftelsesloesning::Arbeidssoekerregisteret,
        );
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav)
            .await;

        let mut tx_1 = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx_1, &message).await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let mut tx_2 = context.start_tx().await;
        let paavegneav_rows_1 =
            bekreftelse_paavegneav::select_by_periode_id(&mut tx_2, &periode_id)
                .await
                .expect("Kunne ikke hente bekreftelse");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

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
        let periode_id = context.periode_id;

        let paavegneav = create_dummy_paavegneav_start(
            periode_id,
            Bekreftelsesloesning::FriskmeldtTilArbeidsformidling,
        );
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav)
            .await;

        let mut tx_1 = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx_1, &message).await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let mut tx_2 = context.start_tx().await;
        let paavegneav_rows_1 =
            bekreftelse_paavegneav::select_by_periode_id(&mut tx_2, &periode_id)
                .await
                .expect("Kunne ikke hente bekreftelse");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

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
        let periode_id = context.periode_id;

        let paavegneav = create_dummy_paavegneav_stopp(
            periode_id,
            Bekreftelsesloesning::Arbeidssoekerregisteret,
        );
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav)
            .await;

        let mut tx_1 = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx_1, &message).await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let mut tx_2 = context.start_tx().await;
        let paavegneav_rows_1 =
            bekreftelse_paavegneav::select_by_periode_id(&mut tx_2, &periode_id)
                .await
                .expect("Kunne ikke hente bekreftelse");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

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
                avro_generator: AvroGenerator::new(schema_registry_settings.clone()),
                processor: BekreftelsePaaVegneAvProcessor::new(schema_registry_settings.clone()),
                periode_id: Uuid::new_v4(),
            }
        })
        .await
    }

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        avro_generator: AvroGenerator,
        processor: BekreftelsePaaVegneAvProcessor,
        periode_id: Uuid,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }
    }
}
