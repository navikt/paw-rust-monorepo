use crate::logic::mutation::{arbeidssoeker_mutation, kartlegging_mutation, periode_mutation};
use crate::model::dao::arbeidssoeker;
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::navn::Navn;
use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_key_gen_client::model::IdentitetType;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use pdl_client::client::PDLClient;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;

pub struct PeriodeProcessor {
    pub deserializer: AvroDeserializer,
    pub key_gen_client: Arc<PawKeyGenClient>,
    pub pdl_client: Arc<PDLClient>,
}

impl PeriodeProcessor {
    pub fn new(
        schema_registry_settings: SrSettings,
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
    ) -> Self {
        Self {
            deserializer: AvroDeserializer::new(schema_registry_settings),
            key_gen_client,
            pdl_client,
        }
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: Periode = self.deserializer.deserialize(payload).await.map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;

        tracing::info!("Mottok hendelse: {:?}", &hendelse);

        // Kafka Key Gen data
        let identiteter_response = self
            .key_gen_client
            .finn_identiteter(hendelse.identitetsnummer.clone())
            .await?;
        let aktor_ider = identiteter_response.filter_by_type(IdentitetType::Aktorid);
        let aktor_id = aktor_ider
            .iter()
            .find(|&i| i.gjeldende)
            .expect("Fant ingen gjeldende aktør-id");
        let arbeidssoeker_id = identiteter_response
            .arbeidssoeker_id
            .expect("Fant ingen arbeidssøker-id");
        let identiteter = identiteter_response.filter_by_type(IdentitetType::Folkeregisterident);
        let identitet = identiteter
            .iter()
            .find(|&i| i.gjeldende)
            .expect("Fant ingen gjeldende identitet");
        let identitetsnummer = Identitetsnummer::new(identitet.identitet.clone())
            .expect("Kunne ikke lage identitetsnummer fra identitet");

        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(tx, &arbeidssoeker_id).await?;

        let parent_id = if arbeidssoeker_rows.is_empty() {
            // PDL data
            let pdl_navn_response = self.pdl_client.hent_person_navn(identitetsnummer).await?;
            let pdl_navn = pdl_navn_response.expect("Fant ingen person for identitetsnummer i PDL");
            let navn = if pdl_navn.navn.is_empty() {
                tracing::warn!("Fant ingen navn for person i PDL, setter alle navn til null");
                Navn::default()
            } else {
                let navn = pdl_navn
                    .navn
                    .first()
                    .expect("Fant ingen navn for person i PDL");
                Navn::new(
                    navn.fornavn.clone(),
                    navn.mellomnavn.clone(),
                    navn.etternavn.clone(),
                )
            };

            let arbeidssoeker = Arbeidssoeker {
                aktor_id: aktor_id.identitet.clone(),
                arbeidssoeker_id,
                identitetsnummer: identitet.identitet.clone(),
                fornavn: navn.fornavn,
                mellomnavn: navn.mellomnavn,
                etternavn: navn.etternavn,
                ledighetsperioder: vec![],
                kontortilknytninger: vec![],
            };
            arbeidssoeker_mutation::lagre_dto(tx, &arbeidssoeker).await?
        } else {
            let arbeidssoeker_row = arbeidssoeker_rows.first().unwrap();
            arbeidssoeker_row.id
        };

        kartlegging_mutation::lagre_hendelse(tx, parent_id, &hendelse).await?;
        periode_mutation::lagre_hendelse(tx, &hendelse).await?;

        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use crate::logic::process::periode_process::PeriodeProcessor;
    use crate::model::dao::{arbeidssoeker, kartlegging, periode};
    use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
    use eksterne_hendelser::serde::AvroSerializer;
    use kafka_key_gen_mock::{default_kafka_key_gen_mock_responses, init_kafka_key_gen_mock};
    use mockito::{Mock, Server, ServerGuard};
    use paw_key_gen_client::client::PawKeyGenClient;
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use pdl_client::client::PDLClient;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use serde::Serialize;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_avsluttet_periode, create_dummy_startet_periode,
    };
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        avro_serializer: AvroSerializer,
        processor: PeriodeProcessor,
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
            let mut mockito_server = Server::new_async().await;

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
                avro_serializer: AvroSerializer::new(schema_registry_settings.clone()),
                processor: PeriodeProcessor::new(
                    schema_registry_settings.clone(),
                    key_gen_client,
                    pdl_client,
                ),
            }
        })
        .await
    }

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        let arbeidssoeker_id = 12345i64;
        let identitetsnummer = "01017012345";
        let periode_id = Uuid::new_v4();
        let periode_1 = create_dummy_startet_periode(identitetsnummer, periode_id);
        let payload_1 = context
            .create_avro_payload(PAW_PERIODE_TOPIC, periode_1.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx, &payload_1).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let mut tx = context.start_tx().await;
        let arbeidssoeker_rows_1 =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows_1 = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row_1 = periode::select_by_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row_1.is_some());
        let periode_row_1 = optional_periode_row_1.expect("Ingen periode funnet");
        assert_eq!(periode_row_1.id, periode_1.id);
        assert_eq!(periode_row_1.identitetsnummer, periode_1.identitetsnummer);
        assert!(periode_row_1.avsluttet_tidspunkt.is_none());

        assert_eq!(arbeidssoeker_rows_1.len(), 1);
        let arbeidssoeker_row_1 = arbeidssoeker_rows_1
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(arbeidssoeker_row_1.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(arbeidssoeker_row_1.identitetsnummer, identitetsnummer);
        assert_eq!(arbeidssoeker_row_1.identitetsnummer, identitetsnummer);

        assert_eq!(kartlegging_rows_1.len(), 1);
        let kartlegging_row_1 = kartlegging_rows_1
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row_1.parent_id, arbeidssoeker_row_1.id);
        assert_eq!(kartlegging_row_1.periode_id, periode_row_1.id);
        assert_eq!(
            kartlegging_row_1.arbeidssoeker_siden,
            periode_row_1.startet_tidspunkt
        );
        assert!(kartlegging_row_1.arbeidsledig_siden.is_none());

        let periode_2 = create_dummy_avsluttet_periode("41017012345", periode_id);
        let payload_2 = context
            .create_avro_payload(PAW_PERIODE_TOPIC, periode_2.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result_2 = context.processor.process_payload(&mut tx, &payload_2).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_2.is_ok());

        let mut tx = context.start_tx().await;
        let arbeidssoeker_rows_2 =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows_2 = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row_2 = periode::select_by_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_periode_row_2.is_some());
        let periode_row_2 = optional_periode_row_2.expect("Ingen periode funnet");
        assert_eq!(periode_row_2.id, periode_2.id);
        assert_eq!(periode_row_2.identitetsnummer, periode_2.identitetsnummer);
        assert!(periode_row_2.avsluttet_tidspunkt.is_some());
        assert_eq!(
            periode_row_2
                .avsluttet_tidspunkt
                .expect("Ingen periode avsluttet_tidspunkt funnet"),
            periode_2
                .avsluttet
                .expect("Ingen periode avsluttet funnet")
                .tidspunkt
        );

        assert_eq!(arbeidssoeker_rows_2.len(), 1);
        let arbeidssoeker_row_2 = arbeidssoeker_rows_2
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(arbeidssoeker_row_2.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(arbeidssoeker_row_2.identitetsnummer, identitetsnummer);
        assert_eq!(arbeidssoeker_row_2.identitetsnummer, identitetsnummer);

        assert_eq!(kartlegging_rows_2.len(), 1);
        let kartlegging_row_2 = kartlegging_rows_2
            .first()
            .expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row_2.parent_id, arbeidssoeker_row_2.id);
        assert_eq!(kartlegging_row_2.periode_id, periode_row_2.id);
        assert_eq!(
            kartlegging_row_2.arbeidssoeker_siden,
            periode_row_2.startet_tidspunkt
        );
        assert!(kartlegging_row_2.arbeidsledig_siden.is_none());
    }
}
