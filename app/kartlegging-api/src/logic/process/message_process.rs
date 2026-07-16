use crate::kafka::error::OversiktProcessorError;
use crate::logic::process::bekreftelse_paavegneav_process::BekreftelsePaaVegneAvProcessor;
use crate::logic::process::bekreftelse_process::BekreftelseProcessor;
use crate::logic::process::egenvurdering_process::EgenvurderingProcessor;
use crate::logic::process::oppfolgingsperiode_process::OppfolgingsperiodeProcessor;
use crate::logic::process::opplysninger_process::OpplysningerProcessor;
use crate::logic::process::periode_process::PeriodeProcessor;
use crate::logic::process::profilering_process::ProfileringProcessor;
use dab_oppfolgingperioder::oppfolgingsperiode::POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC;
use eksterne_hendelser::bekreftelse::bekreftelse::PAW_BEKREFTELSE_TOPIC;
use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
use eksterne_hendelser::egenvurdering::PAW_EGENVURDERING_TOPIC;
use eksterne_hendelser::opplysninger::PAW_OPPLYSNINGER_TOPIC;
use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
use eksterne_hendelser::profilering::PAW_PROFILERING_TOPIC;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use pdl_client::client::PDLClient;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::pin::Pin;
use std::sync::Arc;
use tracing::Instrument;

pub trait MessageProcessorTrait {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>>;
}

pub struct KartleggingMessageProcessor {
    periode_processor: Arc<PeriodeProcessor>,
    opplysninger_processor: Arc<OpplysningerProcessor>,
    profilering_processor: Arc<ProfileringProcessor>,
    egenvurdering_processor: Arc<EgenvurderingProcessor>,
    bekreftelse_processor: Arc<BekreftelseProcessor>,
    bekreftelse_paavegneav_processor: Arc<BekreftelsePaaVegneAvProcessor>,
    oppfolgingsperiode_processor: Arc<OppfolgingsperiodeProcessor>,
}

impl KartleggingMessageProcessor {
    pub fn new(
        schema_registry_settings: SrSettings,
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            periode_processor: Arc::new(PeriodeProcessor::new(
                schema_registry_settings.clone(),
                key_gen_client,
                pdl_client,
            )),
            opplysninger_processor: Arc::new(OpplysningerProcessor::new(
                schema_registry_settings.clone(),
            )),
            profilering_processor: Arc::new(ProfileringProcessor::new(
                schema_registry_settings.clone(),
            )),
            egenvurdering_processor: Arc::new(EgenvurderingProcessor::new(
                schema_registry_settings.clone(),
            )),
            bekreftelse_processor: Arc::new(BekreftelseProcessor::new(
                schema_registry_settings.clone(),
            )),
            bekreftelse_paavegneav_processor: Arc::new(BekreftelsePaaVegneAvProcessor::new(
                schema_registry_settings.clone(),
            )),
            oppfolgingsperiode_processor: Arc::new(OppfolgingsperiodeProcessor::new()),
        })
    }
}

impl MessageProcessor for KartleggingMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                tracing::info!(
                    "Mottok melding på topic: {}, partition: {}, offset: {}",
                    message.topic(),
                    message.partition(),
                    message.offset()
                );
                match (message.topic(), message.payload()) {
                    (topic, None) => Err(OversiktProcessorError::NoPayload {
                        topic: topic.to_string(),
                        partition: message.partition(),
                        offset: message.offset(),
                    }
                    .into()),
                    (topic, Some(payload)) if topic == PAW_PERIODE_TOPIC => {
                        self.periode_processor.process_payload(tx, payload).await
                    }
                    (topic, Some(payload)) if topic == PAW_OPPLYSNINGER_TOPIC => {
                        self.opplysninger_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_PROFILERING_TOPIC => {
                        self.profilering_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_EGENVURDERING_TOPIC => {
                        self.egenvurdering_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_BEKREFTELSE_TOPIC => {
                        self.bekreftelse_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC => {
                        self.bekreftelse_paavegneav_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC => {
                        self.oppfolgingsperiode_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, _) => {
                        panic!("Mottok melding på ukjent topic: {}", topic);
                    }
                }
            }
            .instrument(tracing::Span::current()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::message_process::KartleggingMessageProcessor;
    use crate::model::dao::{arbeidssoeker, kartlegging, periode};
    use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
    use eksterne_hendelser::serde::AvroSerializer;
    use kafka_key_gen_mock::{default_kafka_key_gen_mock_responses, init_kafka_key_gen_mock};
    use mockito::{Mock, Server, ServerGuard};
    use paw_key_gen_client::client::PawKeyGenClient;
    use paw_rdkafka_hwm::hwm_message_processor::MessageProcessor;
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use pdl_client::client::PDLClient;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use rdkafka::message::OwnedMessage;
    use rdkafka::Timestamp;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use serde::Serialize;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use test_data_generator::eksterne_hendelser::create_dummy_startet_periode;
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
        processor: KartleggingMessageProcessor,
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

        async fn create_avro_message(
            &self,
            topic: &'static str,
            payload: impl Serialize,
        ) -> OwnedMessage {
            let payload = self.create_avro_payload(topic, payload).await;
            OwnedMessage::new(
                Some(payload),
                Some("dummy-key".as_bytes().to_vec()),
                topic.to_string(),
                Timestamp::now(),
                0,
                0,
                None,
            )
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
                processor: KartleggingMessageProcessor::new(
                    schema_registry_settings.clone(),
                    key_gen_client,
                    pdl_client,
                )
                .expect("Kunne ikke opprette prosessor"),
            }
        })
        .await
    }

    #[should_panic]
    #[tokio::test]
    async fn test_process_message_ukjent_topic() {
        let context = init().await;

        let message = OwnedMessage::new(
            Some("dummy-payload".as_bytes().to_vec()),
            Some("dummy-key".as_bytes().to_vec()),
            "dummy-topic".to_string(),
            Timestamp::now(),
            0,
            0,
            None,
        );

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result.is_err_and(|e| {
            e.to_string()
                .contains("Mottok melding på ukjent topic: dummy-topic")
        }));
    }

    #[tokio::test]
    async fn test_process_message_periode_start() {
        let context = init().await;

        let arbeidssoeker_id = 12345i64;
        let identitetsnummer = "01017012345";
        let periode_id = Uuid::new_v4();
        let periode_1 = create_dummy_startet_periode(identitetsnummer, periode_id);
        let message_1 = context
            .create_avro_message(PAW_PERIODE_TOPIC, periode_1.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_message(&mut tx, &message_1).await;
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
    }
}
