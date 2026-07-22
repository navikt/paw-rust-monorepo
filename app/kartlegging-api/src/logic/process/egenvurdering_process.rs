use crate::logic::process::PayloadProcessor;
use crate::model::dao::egenvurdering;
use crate::model::dao::egenvurdering::EgenvurderingRow;
use crate::model::error::{DaoError, PayloadProcessorError};
use eksterne_hendelser::egenvurdering::Egenvurdering;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};

pub struct EgenvurderingProcessor {
    pub deserializer: AvroDeserializer,
}

impl EgenvurderingProcessor {
    pub fn new(schema_registry_settings: SrSettings) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_settings),
        }
    }
}

impl PayloadProcessor for EgenvurderingProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<(), ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: Egenvurdering = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok Egenvurdering-hendelse");

                let row = EgenvurderingRow::new(
                    hendelse.id,
                    hendelse.periode_id,
                    hendelse.profilering_id,
                    hendelse.profilert_til.as_ref().to_string(),
                    hendelse.egenvurdering.as_ref().to_string(),
                    hendelse.sendt_inn_av.tidspunkt,
                );
                let count = egenvurdering::count_by_id(tx, &hendelse.id).await?;
                if count > 1 {
                    Err(DaoError::multiple_rows(message, "egenvurderinger", count as usize).into())
                } else if count == 1 {
                    egenvurdering::update(tx, &row).await?;
                    Ok(())
                } else {
                    egenvurdering::insert(tx, &row).await?;
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::egenvurdering_process::EgenvurderingProcessor;
    use crate::logic::process::PayloadProcessor;
    use crate::model::dao::egenvurdering;
    use eksterne_hendelser::egenvurdering::PAW_EGENVURDERING_TOPIC;
    use eksterne_hendelser::vo::profilert_til::ProfilertTil;
    use mockito::{Mock, Server, ServerGuard};
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use sqlx::{PgPool, Postgres, Transaction};
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::eksterne_hendelser::create_dummy_egenvurdering;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        let identitetsnummer = "01017012345";
        let periode_id = Uuid::new_v4();
        let profilering_id = Uuid::new_v4();
        let egenvurdering_id = Uuid::new_v4();
        let egenvurdering = create_dummy_egenvurdering(
            identitetsnummer,
            periode_id,
            profilering_id,
            egenvurdering_id,
        );
        let message = context
            .avro_generator
            .create_avro_message(PAW_EGENVURDERING_TOPIC, egenvurdering)
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        assert!(result.is_ok());
        let optional_row = egenvurdering::select_by_id(&mut tx, &egenvurdering_id)
            .await
            .expect("Kunne ikke hente egenvurdering");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_row.is_some());
        let row = optional_row.expect("Ingen egenvurdering funnet");
        assert_eq!(row.id, egenvurdering_id);
        assert_eq!(row.periode_id, periode_id);
        assert_eq!(row.profilering_id, profilering_id);
        assert_eq!(
            row.profilert_til,
            ProfilertTil::AntattGodeMuligheter.as_ref().to_string()
        );
        assert_eq!(
            row.egenvurdert_til,
            ProfilertTil::OppgittHindringer.as_ref().to_string()
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
                processor: EgenvurderingProcessor::new(schema_registry_settings.clone()),
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
        processor: EgenvurderingProcessor,
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
