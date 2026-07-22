use crate::logic::process::PayloadProcessor;
use crate::model::dao::bekreftelse::BekreftelseRow;
use crate::model::dao::kartlegging::KartleggingRow;
use crate::model::dao::{bekreftelse, kartlegging};
use crate::model::error::{DaoError, PayloadProcessorError};
use chrono::{DateTime, Utc};
use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};

pub struct BekreftelseProcessor {
    pub deserializer: AvroDeserializer,
}

impl BekreftelseProcessor {
    pub fn new(schema_registry_settings: SrSettings) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_settings),
        }
    }

    async fn lagre_bekreftelse<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &OwnedMessage,
        hendelse: &'a Bekreftelse,
    ) -> anyhow::Result<u64> {
        let row = BekreftelseRow::new(
            hendelse.id,
            hendelse.periode_id,
            hendelse.svar.gjelder_fra,
            hendelse.svar.gjelder_til,
            hendelse.svar.har_jobbet_i_denne_perioden,
            hendelse.svar.vil_fortsette_som_arbeidssoeker,
            hendelse.bekreftelsesloesning.as_ref().to_string(),
            hendelse.svar.sendt_inn_av.tidspunkt,
        );
        let count = bekreftelse::count_by_id(tx, &hendelse.id).await?;
        if count > 1 {
            Err(DaoError::multiple_rows(message, "bekreftelser", count as usize).into())
        } else if count == 1 {
            bekreftelse::update(tx, &row).await
        } else {
            bekreftelse::insert(tx, &row).await
        }
    }

    fn utled_arbeidsledighet_fra_bekreftelse<'a>(
        &self,
        hendelse: &Bekreftelse,
        kartlegging_row: &KartleggingRow,
    ) -> Option<DateTime<Utc>> {
        if hendelse.svar.har_jobbet_i_denne_perioden {
            // Nuller ut ledighet hvis arbeidssøker har jobbet
            None
        } else {
            match kartlegging_row.arbeidsledig_fra {
                // Har ikke jobbet og ledighet er ikke satt, så benytt bekreftelse gjelder_fra
                None => Some(hendelse.svar.gjelder_fra),
                // Har ikke jobbet og ledighet er satt, så behold eksisterende ledighet
                Some(arbeidsledig_fra) => Some(arbeidsledig_fra),
            }
        }
    }
}

impl PayloadProcessor for BekreftelseProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<(), ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: Bekreftelse = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok Bekreftelse-hendelse");

                self.lagre_bekreftelse(tx, &message, &hendelse).await?;

                let kartlegging_rows =
                    kartlegging::select_by_periode_id(tx, &hendelse.periode_id).await?;
                let count = kartlegging_rows.len();
                if count > 1 {
                    Err(DaoError::multiple_rows(message, "kartlegginger", count).into())
                } else if count == 1 {
                    let kartlegging_row = kartlegging_rows
                        .first()
                        .ok_or_else(|| DaoError::no_rows(message, "kartlegginger"))?;
                    let arbeidssoeker_til = kartlegging_row.arbeidssoeker_til;
                    let arbeidsledig_fra =
                        self.utled_arbeidsledighet_fra_bekreftelse(&hendelse, &kartlegging_row);

                    kartlegging::update(
                        tx,
                        &hendelse.periode_id,
                        &arbeidssoeker_til,
                        &arbeidsledig_fra,
                    )
                    .await?;

                    Ok(())
                } else {
                    tracing::warn!("Fant ingen kartlegginger for periode-id");
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::read_app_config;
    use crate::logic::process::bekreftelse_process::BekreftelseProcessor;
    use crate::logic::process::periode_process::PeriodeProcessor;
    use crate::logic::process::PayloadProcessor;
    use crate::model::dao::{arbeidssoeker, bekreftelse, kartlegging, periode};
    use eksterne_hendelser::bekreftelse::bekreftelse::PAW_BEKREFTELSE_TOPIC;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
    use kafka_key_gen_mock::{default_kafka_key_gen_mock_responses, init_kafka_key_gen_mock};
    use mockito::{Mock, Server, ServerGuard};
    use paw_key_gen_client::client::PawKeyGenClient;
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use pdl_client::client::PDLClient;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_bekreftelse, create_dummy_startet_periode,
    };
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;
    use tracing_test::traced_test;
    use uuid::Uuid;

    #[traced_test]
    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        test_process_periode(context).await;
        test_process_bekreftelse_1_med_periode(context).await;
        test_process_bekreftelse_2_med_periode(context).await;
        test_process_bekreftelse_3_uten_periode(context).await;
    }

    async fn test_process_periode(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer = context.identitetsnummer_1;
        let periode_id = context.periode_id_1;

        let periode = create_dummy_startet_periode(identitetsnummer, periode_id);
        let message = context
            .avro_generator
            .create_avro_message(PAW_PERIODE_TOPIC, periode)
            .await;

        let mut tx_1 = context.start_tx().await;
        let result = context
            .periode_processor
            .process_payload(&mut tx_1, &message)
            .await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        eprintln!("Result: {:?}", result);
        assert!(result.is_ok());

        let mut tx_2 = context.start_tx().await;
        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx_2, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx_2, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx_2, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row.is_some());
        let periode_row = optional_periode_row.expect("Ingen periode funnet");
        assert_eq!(periode_row.id, periode_id);
        assert_eq!(periode_row.identitetsnummer, identitetsnummer);
        assert!(periode_row.avsluttet_tidspunkt.is_none());

        assert_eq!(arbeidssoeker_rows.len(), 1);
        let arbeidssoeker_row = arbeidssoeker_rows
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(arbeidssoeker_row.id, arbeidssoeker_id);
        assert_eq!(arbeidssoeker_row.identitetsnummer, identitetsnummer);

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert!(kartlegging_row.arbeidssoeker_til.is_none());
        assert!(kartlegging_row.arbeidsledig_fra.is_none());
    }

    async fn test_process_bekreftelse_1_med_periode(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer = context.identitetsnummer_1;
        let periode_id = context.periode_id_1;
        let bekreftelse_id = context.bekreftelse_id_1;

        let bekreftelse =
            create_dummy_bekreftelse(identitetsnummer, periode_id, bekreftelse_id, false, true);
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_TOPIC, bekreftelse)
            .await;

        let mut tx_1 = context.start_tx().await;
        let result = context
            .bekreftelse_processor
            .process_payload(&mut tx_1, &message)
            .await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result.is_ok());

        let mut tx_2 = context.start_tx().await;
        let optional_bekreftelse_row = bekreftelse::select_by_id(&mut tx_2, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_bekreftelse_row.is_some());
        let bekreftelse_row = optional_bekreftelse_row.expect("Ingen bekreftelse funnet");
        assert_eq!(bekreftelse_row.id, bekreftelse_id);
        assert_eq!(bekreftelse_row.periode_id, periode_id);
        assert_eq!(
            bekreftelse_row.bekreftelsesloesning,
            Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string()
        );

        let mut tx_3 = context.start_tx().await;
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx_3, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx_3, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx_3.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row.is_some());
        let periode_row = optional_periode_row.expect("Ingen periode funnet");
        assert_eq!(periode_row.id, periode_id);
        assert_eq!(periode_row.identitetsnummer, identitetsnummer);
        assert!(periode_row.avsluttet_tidspunkt.is_none());

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert!(kartlegging_row.arbeidssoeker_til.is_none());
        assert!(kartlegging_row.arbeidsledig_fra.is_some());
        let arbeidsledig_fra = kartlegging_row
            .arbeidsledig_fra
            .expect("Kunne ikke hente ledighet");
        assert_eq!(arbeidsledig_fra, bekreftelse_row.gjelder_fra);
    }

    async fn test_process_bekreftelse_2_med_periode(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer = context.identitetsnummer_1;
        let periode_id = context.periode_id_1;
        let bekreftelse_id = context.bekreftelse_id_2;

        let bekreftelse =
            create_dummy_bekreftelse(identitetsnummer, periode_id, bekreftelse_id, true, true);
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_TOPIC, bekreftelse)
            .await;

        let mut tx_1 = context.start_tx().await;
        let result = context
            .bekreftelse_processor
            .process_payload(&mut tx_1, &message)
            .await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result.is_ok());

        let mut tx_2 = context.start_tx().await;
        let optional_bekreftelse_row = bekreftelse::select_by_id(&mut tx_2, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_bekreftelse_row.is_some());
        let bekreftelse_row = optional_bekreftelse_row.expect("Ingen bekreftelse funnet");
        assert_eq!(bekreftelse_row.id, bekreftelse_id);
        assert_eq!(bekreftelse_row.periode_id, periode_id);
        assert_eq!(
            bekreftelse_row.bekreftelsesloesning,
            Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string()
        );

        let mut tx_3 = context.start_tx().await;
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx_3, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx_3, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx_3.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row.is_some());
        let periode_row = optional_periode_row.expect("Ingen periode funnet");
        assert_eq!(periode_row.id, periode_id);
        assert_eq!(periode_row.identitetsnummer, identitetsnummer);
        assert!(periode_row.avsluttet_tidspunkt.is_none());

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert!(kartlegging_row.arbeidssoeker_til.is_none());
        assert!(kartlegging_row.arbeidsledig_fra.is_none());
    }

    async fn test_process_bekreftelse_3_uten_periode(context: &TestContext) {
        let identitetsnummer = context.identitetsnummer_3;
        let periode_id = context.periode_id_3;
        let bekreftelse_id = context.bekreftelse_id_3;

        let mut tx_1 = context.start_tx().await;
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx_1, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx_1, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(kartlegging_rows.is_empty());
        assert!(optional_periode_row.is_none());

        let bekreftelse =
            create_dummy_bekreftelse(identitetsnummer, periode_id, bekreftelse_id, false, true);
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_TOPIC, bekreftelse)
            .await;

        let mut tx_2 = context.start_tx().await;
        let result = context
            .bekreftelse_processor
            .process_payload(&mut tx_2, &message)
            .await;
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result.is_ok());

        let mut tx_3 = context.start_tx().await;
        let optional_row = bekreftelse::select_by_id(&mut tx_3, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx_3.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_row.is_some());
        let row = optional_row.expect("Ingen bekreftelse funnet");
        assert_eq!(row.id, bekreftelse_id);
        assert_eq!(row.periode_id, periode_id);
        assert_eq!(
            row.bekreftelsesloesning,
            Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string()
        );
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let mut mockito_server = Server::new_async().await;

            let app_config = Arc::new(read_app_config().expect("Kunne ikke lese app_config.yaml"));

            let schema_registry_guard = create_schema_registry_mock(&mut mockito_server)
                .await
                .expect("Failed to create schema registry mock");
            let schema_registry_settings = schema_registry_guard.schema_registry_settings;

            let kafka_key_gen_mock_responses = default_kafka_key_gen_mock_responses();
            let kafka_key_gen_mock_guard =
                init_kafka_key_gen_mock(&mut mockito_server, kafka_key_gen_mock_responses)
                    .await
                    .expect("Kunne ikke initialisere Kafka Key Gen mock");

            let pdl_mock_responses = default_pdl_mock_responses();
            let pdl_mock_guard = init_pdl_mock(&mut mockito_server, pdl_mock_responses)
                .await
                .expect("Kunne ikke initialisere PDL mock server");

            let mut schema_registry_mocks = schema_registry_guard.mocks;
            let mut kafka_key_gen_mocks = kafka_key_gen_mock_guard.mocks;
            let mut mocks = pdl_mock_guard.mocks;
            mocks.append(&mut schema_registry_mocks);
            mocks.append(&mut kafka_key_gen_mocks);

            let http_client = reqwest::Client::builder()
                .no_proxy()
                .build()
                .expect("Failed to build reqwest client");

            let key_gen_client = Arc::new(PawKeyGenClient::new(
                mockito_server.url(),
                "test-scope".to_string(),
                http_client.clone(),
                Arc::new(TokenClientStub::new()),
            ));

            let pdl_client = Arc::new(PDLClient::new(
                "test-scope".to_string(),
                format!("{}/pdl", mockito_server.url()),
                http_client.clone(),
                Arc::new(TokenClientStub::new()),
            ));

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
                periode_processor: PeriodeProcessor::new(
                    app_config,
                    schema_registry_settings.clone(),
                    key_gen_client,
                    pdl_client,
                ),
                bekreftelse_processor: BekreftelseProcessor::new(schema_registry_settings.clone()),
                arbeidssoeker_id_1: 12345,
                identitetsnummer_1: "01017012345",
                identitetsnummer_3: "02017012345",
                periode_id_1: Uuid::new_v4(),
                periode_id_3: Uuid::new_v4(),
                bekreftelse_id_1: Uuid::new_v4(),
                bekreftelse_id_2: Uuid::new_v4(),
                bekreftelse_id_3: Uuid::new_v4(),
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
        periode_processor: PeriodeProcessor,
        bekreftelse_processor: BekreftelseProcessor,
        arbeidssoeker_id_1: i64,
        identitetsnummer_1: &'static str,
        identitetsnummer_3: &'static str,
        periode_id_1: Uuid,
        periode_id_3: Uuid,
        bekreftelse_id_1: Uuid,
        bekreftelse_id_2: Uuid,
        bekreftelse_id_3: Uuid,
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
