use crate::config::AppConfig;
use crate::logic::process::PayloadProcessor;
use crate::logic::process::kartlegging_process::{
    utled_arbeidsledighet_for_periode_med_aktiv_kartlegging, utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging,
};
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::dao::kartlegging::KartleggingRow;
use crate::model::dao::periode::PeriodeRow;
use crate::model::dao::{arbeidssoeker, kartlegging, periode};
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::navn::Navn;
use crate::model::error::{DaoError, IdentityError, PayloadProcessorError};
use crate::model::result::ProcessorResult;
use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::AvroDeserializer;
use eksterne_hendelser::vo::metadata::Metadata;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_key_gen_client::model::IdentitetType;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use pdl_client::client::PDLClient;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;

pub struct PeriodeProcessor {
    pub app_config: Arc<AppConfig>,
    pub deserializer: AvroDeserializer,
    pub key_gen_client: Arc<PawKeyGenClient>,
    pub pdl_client: Arc<PDLClient>,
}

impl PeriodeProcessor {
    pub fn new(
        app_config: Arc<AppConfig>,
        schema_registry_settings: SrSettings,
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
    ) -> Self {
        Self {
            app_config,
            deserializer: AvroDeserializer::new(schema_registry_settings),
            key_gen_client,
            pdl_client,
        }
    }

    async fn lagre_periode<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
        hendelse: &'a Periode,
    ) -> anyhow::Result<u64> {
        let row = PeriodeRow::new(
            hendelse.id,
            hendelse.identitetsnummer.clone(),
            hendelse.startet.tidspunkt().to_owned(),
            hendelse
                .avsluttet
                .as_ref()
                .map(|metadata| metadata.tidspunkt().to_owned()),
        );
        let count = periode::count_by_id(tx, &hendelse.id).await?;
        if count > 1 {
            // Mer enn én arbeidssøker funnet for arbeidssøker-id
            Err(DaoError::multiple_rows(message, "perioder", count as usize).into())
        } else if count == 1 {
            periode::update(tx, &row).await
        } else {
            periode::insert(tx, &row).await
        }
    }

    async fn lagre_arbeidssoker<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
        hendelse: &'a Periode,
    ) -> anyhow::Result<Arbeidssoeker> {
        // Hent identiteter fra Kafka Key Gen
        let arbeidssoeker = self
            .hent_identiteter(message, &hendelse.identitetsnummer)
            .await?;

        // Søk etter arbeidssøker(e)
        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(tx, &arbeidssoeker.id).await?;

        if arbeidssoeker_rows.len() > 1 {
            // Mer enn én arbeidssøker funnet
            Err(DaoError::multiple_rows(message, "arbeidssøkere", arbeidssoeker_rows.len()).into())
        } else if arbeidssoeker_rows.len() == 1 {
            // Arbeidssøker finnes fra før

            Ok(arbeidssoeker)
        } else {
            // Arbeidssøker finnes ikke fra før

            // Hent navn fra PDL
            let navn = self
                .hent_navn(message, &arbeidssoeker.identitetsnummer)
                .await?;

            // Lagre ny arbeidssøker
            let arbeidssoeker_row = ArbeidssoekerRow::new(
                arbeidssoeker.id,
                arbeidssoeker.aktor_id.clone(),
                arbeidssoeker.identitetsnummer.clone(),
                navn.fornavn.clone(),
                navn.mellomnavn.clone(),
                navn.etternavn.clone(),
            );
            arbeidssoeker::insert(tx, &arbeidssoeker_row).await?;

            Ok(arbeidssoeker)
        }
    }

    async fn lagre_kartlegging<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
        hendelse: &'a Periode,
        arbeidssoeker: &'a Arbeidssoeker,
    ) -> anyhow::Result<u64> {
        // Søk etter kartlegging(er)
        let kartlegging_rows = kartlegging::select_by_periode_id(tx, &hendelse.id).await?;

        if kartlegging_rows.len() > 1 {
            // Mer enn én kartlegging funnet
            Err(DaoError::multiple_rows(message, "kartlegginger", kartlegging_rows.len()).into())
        } else if kartlegging_rows.len() == 1 {
            // Kartlegging finnes fra før

            let kartlegging_row = kartlegging_rows
                .first()
                .ok_or_else(|| DaoError::no_rows(message, "kartlegginger"))?;

            let arbeidssoeker_til = hendelse
                .avsluttet
                .as_ref()
                .map(|metadata| metadata.tidspunkt().to_owned());

            // Beregn ledighet fra aktiv kartlegging
            let arbeidsledig_fra = utled_arbeidsledighet_for_periode_med_aktiv_kartlegging(
                tx,
                &arbeidssoeker.id,
                &hendelse.id,
                &hendelse.startet.tidspunkt,
                &kartlegging_row.arbeidsledig_fra,
                self.app_config.periode_gap_grense_for_ledighet,
            )
            .await?;

            // Lagre eksisterende kartlegging med arbeidssoeker_til og arbeidsledig_fra
            kartlegging::update(tx, &hendelse.id, &arbeidssoeker_til, &arbeidsledig_fra).await
        } else {
            // Kartlegging finnes ikke fra før

            let arbeidssoeker_fra = hendelse.startet.tidspunkt().to_owned();
            let arbeidssoeker_til = hendelse
                .avsluttet
                .as_ref()
                .map(|metadata| metadata.tidspunkt().to_owned());

            // Beregn ledighet fra tidligere kartlegging, om den finnes
            let arbeidsledig_fra = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
                tx,
                &arbeidssoeker.id,
                &hendelse.id,
                &arbeidssoeker_fra,
                self.app_config.periode_gap_grense_for_ledighet,
            )
            .await?;

            // Lagre ny kartlegging
            let kartlegging_row = KartleggingRow::new(
                hendelse.id.clone(),
                arbeidssoeker.id,
                arbeidssoeker_fra,
                arbeidssoeker_til,
                arbeidsledig_fra,
            );
            kartlegging::insert(tx, &kartlegging_row).await
        }
    }

    #[tracing::instrument(skip_all)]
    async fn hent_identiteter<'a>(
        &'a self,
        message: &'a OwnedMessage,
        identitetsnummer: &'a String,
    ) -> anyhow::Result<Arbeidssoeker> {
        let identiteter_response = self
            .key_gen_client
            .finn_identiteter(identitetsnummer.clone())
            .await?;
        let arbeidssoeker_id = identiteter_response
            .arbeidssoeker_id
            .ok_or_else(|| IdentityError::not_found(message, IdentitetType::Arbeidssoekerid))?;
        let aktor_ider = identiteter_response.filter_by_type(IdentitetType::Aktorid);
        let aktor_id = aktor_ider
            .iter()
            .find(|&i| i.gjeldende)
            .ok_or_else(|| IdentityError::not_found(message, IdentitetType::Aktorid))?;
        let identiteter = identiteter_response.filter_by_type(IdentitetType::Folkeregisterident);
        let folkeregisterident = identiteter
            .iter()
            .find(|&i| i.gjeldende)
            .ok_or_else(|| IdentityError::not_found(message, IdentitetType::Folkeregisterident))?;
        Ok(Arbeidssoeker::from_identer(
            arbeidssoeker_id,
            aktor_id.identitet.clone(),
            folkeregisterident.identitet.clone(),
        ))
    }

    #[tracing::instrument(skip_all)]
    async fn hent_navn<'a>(
        &'a self,
        message: &'a OwnedMessage,
        identitetsnummer: &String,
    ) -> anyhow::Result<Navn> {
        let identitetsnummer_struct =
            Identitetsnummer::new(identitetsnummer.clone()).ok_or_else(|| {
                PayloadProcessorError::processing_error(
                    message,
                    "Ugyldig identitetsnummer fra kafka-key-gen",
                )
            })?;

        let pdl_navn_response = self
            .pdl_client
            .hent_person_navn(identitetsnummer_struct)
            .await?;
        let pdl_navn = pdl_navn_response.ok_or_else(|| {
            PayloadProcessorError::processing_error(message, "Fant ingen person i PDL")
        })?;

        if pdl_navn.navn.is_empty() {
            tracing::warn!("Fant ingen navn for person i PDL, setter alle navn til null");
            Ok(Navn::default())
        } else {
            let pdl_navn_entry = pdl_navn.navn.first().ok_or_else(|| {
                PayloadProcessorError::processing_error(message, "Fant ingen navn for person i PDL")
            })?;
            Ok(Navn::new(
                pdl_navn_entry.fornavn.clone(),
                pdl_navn_entry.mellomnavn.clone(),
                pdl_navn_entry.etternavn.clone(),
            ))
        }
    }
}

impl PayloadProcessor for PeriodeProcessor {
    #[tracing::instrument(skip_all, fields(topic = %message.topic(), partition = %message.partition(), offset = %message.offset()))]
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<ProcessorResult, ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: Periode = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok {}-hendelse", &hendelse);

                // Lagre periode
                self.lagre_periode(tx, message, &hendelse).await?;

                // Lagre arbeidssøker
                let arbeidssoeker = self.lagre_arbeidssoker(tx, message, &hendelse).await?;

                // Lagre kartlegging
                self.lagre_kartlegging(tx, message, &hendelse, &arbeidssoeker)
                    .await?;

                Ok(ProcessorResult::Continue)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::read_app_config;
    use crate::logic::process::PayloadProcessor;
    use crate::logic::process::periode_process::PeriodeProcessor;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::dao::bekreftelse::BekreftelseRow;
    use crate::model::dao::kartlegging::KartleggingRow;
    use crate::model::dao::{arbeidssoeker, bekreftelse, kartlegging, periode};
    use chrono::{Duration, TimeZone, Utc};
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
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
        create_dummy_avslutt_periode, create_dummy_start_periode,
    };
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;
    use tracing_test::traced_test;
    use uuid::Uuid;

    #[traced_test]
    #[tokio::test]
    async fn test_process_messages() -> anyhow::Result<()> {
        let context = init().await?;
        test_process_periode_1_start(context).await?;
        test_process_periode_1_avsluttet(context).await?;
        test_process_periode_2_start(context).await?;

        Ok(())
    }

    async fn test_process_periode_1_start(context: &TestContext) -> anyhow::Result<()> {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer_1 = context.identitetsnummer_1_1;
        let identitetsnummer_2 = context.identitetsnummer_1_2;
        let periode_id_1 = context.periode_id_1;

        let periode = create_dummy_start_periode(identitetsnummer_1, periode_id_1, None);
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssokerperioder-v1", periode)
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        assert!(result.is_ok());
        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id_1)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id_1)
            .await
            .expect("Kunne ikke hente periode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row.is_some());
        let periode_row = optional_periode_row.expect("Ingen periode funnet");
        assert_eq!(periode_row.id, periode_id_1);
        assert_eq!(periode_row.identitetsnummer, identitetsnummer_1);
        assert!(periode_row.avsluttet_tidspunkt.is_none());

        assert_eq!(arbeidssoeker_rows.len(), 1);
        let arbeidssoeker_row = arbeidssoeker_rows
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(arbeidssoeker_row.id, arbeidssoeker_id);
        assert_eq!(arbeidssoeker_row.identitetsnummer, identitetsnummer_2);

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id_1);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert!(kartlegging_row.arbeidssoeker_til.is_none());
        assert!(kartlegging_row.arbeidsledig_fra.is_none());

        Ok(())
    }

    async fn test_process_periode_1_avsluttet(context: &TestContext) -> anyhow::Result<()> {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer_2 = context.identitetsnummer_1_2;
        let periode_id_1 = context.periode_id_1;

        let periode = create_dummy_avslutt_periode(identitetsnummer_2, periode_id_1, None, None);
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssokerperioder-v1", periode)
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        assert!(result.is_ok());
        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id_1)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id_1)
            .await
            .expect("Kunne ikke hente periode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row.is_some());
        let periode_row = optional_periode_row.expect("Ingen periode funnet");
        assert_eq!(periode_row.id, periode_id_1);
        assert_eq!(periode_row.identitetsnummer, identitetsnummer_2);
        assert!(periode_row.avsluttet_tidspunkt.is_some());

        assert_eq!(arbeidssoeker_rows.len(), 1);
        let arbeidssoeker_row = arbeidssoeker_rows
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(arbeidssoeker_row.id, arbeidssoeker_id);
        assert_eq!(arbeidssoeker_row.identitetsnummer, identitetsnummer_2);

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id_1);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert!(kartlegging_row.arbeidssoeker_til.is_some());
        assert!(kartlegging_row.arbeidsledig_fra.is_none());

        Ok(())
    }

    async fn test_process_periode_2_start(context: &TestContext) -> anyhow::Result<()> {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer_2 = context.identitetsnummer_1_2;
        let periode_id_2 = context.periode_id_2;

        let periode = create_dummy_start_periode(identitetsnummer_2, periode_id_2, None);
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssokerperioder-v1", periode)
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        assert!(result.is_ok());
        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id_2)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id_2)
            .await
            .expect("Kunne ikke hente periode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row.is_some());
        let periode_row = optional_periode_row.expect("Ingen periode funnet");
        assert_eq!(periode_row.id, periode_id_2);
        assert_eq!(periode_row.identitetsnummer, identitetsnummer_2);
        assert!(periode_row.avsluttet_tidspunkt.is_none());

        assert_eq!(arbeidssoeker_rows.len(), 1);
        let arbeidssoeker_row = arbeidssoeker_rows
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(arbeidssoeker_row.id, arbeidssoeker_id);
        assert_eq!(arbeidssoeker_row.identitetsnummer, identitetsnummer_2);
        assert_eq!(arbeidssoeker_row.identitetsnummer, identitetsnummer_2);

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id_2);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert!(kartlegging_row.arbeidssoeker_til.is_none());
        assert!(kartlegging_row.arbeidsledig_fra.is_none());

        Ok(())
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> anyhow::Result<&'static TestContext> {
        let context = INIT
            .get_or_init(|| async {
                let mut mockito_server = Server::new_async().await;

                let app_config =
                    Arc::new(read_app_config().expect("Kunne ikke lese app_config.yaml"));

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

                let postgres_guard = setup_postgres_container()
                    .await
                    .expect("Failed to start Postgres container");
                println!("Migrerer databasemodell");
                sqlx::migrate!("./migrations")
                    .run(&postgres_guard.pg_pool)
                    .await
                    .expect("Failed to run migrations");

                TestContext {
                    mockito_server,
                    mocks,
                    pg_pool: postgres_guard.pg_pool,
                    avro_generator: AvroGenerator::new(schema_registry_settings.clone()),
                    processor: PeriodeProcessor::new(
                        app_config,
                        schema_registry_settings.clone(),
                        key_gen_client,
                        pdl_client,
                    ),
                    arbeidssoeker_id_1: 12345,
                    identitetsnummer_1_1: "41017012345",
                    identitetsnummer_1_2: "01017012345",
                    periode_id_1: Uuid::new_v4(),
                    periode_id_2: Uuid::new_v4(),
                }
            })
            .await;

        Ok(context)
    }

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        avro_generator: AvroGenerator,
        processor: PeriodeProcessor,
        arbeidssoeker_id_1: i64,
        identitetsnummer_1_1: &'static str,
        identitetsnummer_1_2: &'static str,
        periode_id_1: Uuid,
        periode_id_2: Uuid,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            println!(
                "Starter transaksjon (antall ledige tråder: {})",
                self.pg_pool.num_idle()
            );
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }
    }
}
