use crate::config::AppConfig;
use crate::logic::process::bekreftelse_paavegneav_process::BekreftelsePaaVegneAvProcessor;
use crate::logic::process::bekreftelse_process::BekreftelseProcessor;
use crate::logic::process::egenvurdering_process::EgenvurderingProcessor;
use crate::logic::process::oppfolgingsperiode_process::OppfolgingsperiodeProcessor;
use crate::logic::process::opplysninger_process::OpplysningerProcessor;
use crate::logic::process::periode_process::PeriodeProcessor;
use crate::logic::process::profilering_process::ProfileringProcessor;
use crate::logic::process::PayloadProcessor;
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
        app_config: Arc<AppConfig>,
        schema_registry_settings: SrSettings,
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            periode_processor: Arc::new(PeriodeProcessor::new(
                app_config,
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
                tracing::debug!(
                    "Mottok melding på topic: {}, partition: {}, offset: {}",
                    message.topic(),
                    message.partition(),
                    message.offset()
                );

                let result = match message.topic() {
                    topic if topic == PAW_PERIODE_TOPIC => {
                        self.periode_processor.process_payload(tx, message).await
                    }
                    topic if topic == PAW_OPPLYSNINGER_TOPIC => {
                        self.opplysninger_processor
                            .process_payload(tx, message)
                            .await
                    }
                    topic if topic == PAW_PROFILERING_TOPIC => {
                        self.profilering_processor
                            .process_payload(tx, message)
                            .await
                    }
                    topic if topic == PAW_EGENVURDERING_TOPIC => {
                        self.egenvurdering_processor
                            .process_payload(tx, message)
                            .await
                    }
                    topic if topic == PAW_BEKREFTELSE_TOPIC => {
                        self.bekreftelse_processor
                            .process_payload(tx, message)
                            .await
                    }
                    topic if topic == PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC => {
                        self.bekreftelse_paavegneav_processor
                            .process_payload(tx, message)
                            .await
                    }
                    topic if topic == POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC => {
                        self.oppfolgingsperiode_processor
                            .process_payload(tx, message)
                            .await
                    }
                    topic => {
                        panic!("Mottok melding på ukjent topic: {}", topic);
                    }
                };

                result.map_err(|e| {
                    tracing::error!(
                        error = e,
                        "Prosessering av melding på topic: {}, partition: {}, offset: {} feilet",
                        message.topic(),
                        message.partition(),
                        message.offset()
                    );
                    e
                })
            }
            .instrument(tracing::Span::current()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::config::read_app_config;
    use crate::logic::process::message_process::KartleggingMessageProcessor;
    use crate::model::dao::{
        arbeidssoeker, bekreftelse, bekreftelse_paavegneav, egenvurdering, kartlegging,
        kontortilknytning, opplysninger, periode, profilering,
    };
    use crate::model::dto::kontortilknytning::KontorType;
    use crate::model::dto::opplysninger::Jobbsituasjon;
    use crate::model::dto::profilering::ProfilertTil;
    use dab_oppfolgingperioder::oppfolgingsperiode::POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC;
    use eksterne_hendelser::bekreftelse::bekreftelse::PAW_BEKREFTELSE_TOPIC;
    use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use eksterne_hendelser::egenvurdering::PAW_EGENVURDERING_TOPIC;
    use eksterne_hendelser::opplysninger::PAW_OPPLYSNINGER_TOPIC;
    use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
    use eksterne_hendelser::profilering::PAW_PROFILERING_TOPIC;
    use kafka_key_gen_mock::{default_kafka_key_gen_mock_responses, init_kafka_key_gen_mock};
    use mockito::{Mock, Server, ServerGuard};
    use futures::FutureExt;
    use paw_key_gen_client::client::PawKeyGenClient;
    use paw_rdkafka_hwm::hwm_message_processor::MessageProcessor;
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use pdl_client::client::PDLClient;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use rdkafka::message::OwnedMessage;
    use rdkafka::Timestamp;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::dab_oppfolgingsperiode::create_dummy_oppfolgingsperiode_startet;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_bekreftelse, create_dummy_egenvurdering, create_dummy_opplysninger,
        create_dummy_paavegneav_start, create_dummy_profilering, create_dummy_startet_periode,
    };
    use test_data_generator::json::JsonGenerator;
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;
    use tracing_test::traced_test;
    use uuid::Uuid;

    #[traced_test]
    #[tokio::test]
    async fn test_process_illegal_message() {
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
        let result = std::panic::AssertUnwindSafe(
            context.processor.process_message(&mut tx, &message),
        )
        .catch_unwind()
        .await;
        let _ = tx.rollback().await;

        assert!(result.is_err(), "Forventet panic for ukjent topic");
    }

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        test_process_periode(context).await;
        test_process_opplysninger(context).await;
        test_process_profilering(context).await;
        test_process_egenvurdering(context).await;
        test_process_bekreftelse(context).await;
        test_process_bekreftelse_paavegneav(context).await;
        test_process_oppsolgingsperiode(context).await;
    }

    async fn test_process_periode(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id;
        let identitetsnummer = context.identitetsnummer;
        let periode_id = context.periode_id;

        let periode = create_dummy_startet_periode(identitetsnummer, periode_id);
        let message = context
            .avro_generator
            .create_avro_message(PAW_PERIODE_TOPIC, periode.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(&mut tx, &arbeidssoeker_id)
                .await
                .expect("Kunne ikke hente arbeidssøkere");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

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
        assert_eq!(arbeidssoeker_row.identitetsnummer, identitetsnummer);

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen arbeidssøker funnet");
        assert_eq!(kartlegging_row.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(kartlegging_row.periode_id, periode_id);
        assert_eq!(
            kartlegging_row.arbeidssoeker_fra,
            periode_row.startet_tidspunkt
        );
        assert_eq!(
            kartlegging_row.arbeidssoeker_til,
            periode_row.avsluttet_tidspunkt
        );
        assert!(kartlegging_row.arbeidsledig_fra.is_none());
    }

    async fn test_process_opplysninger(context: &TestContext) {
        let identitetsnummer = context.identitetsnummer;
        let periode_id = context.periode_id;
        let opplysninger_id = context.opplysninger_id;

        let opplysninger = create_dummy_opplysninger(identitetsnummer, periode_id, opplysninger_id);
        let message = context
            .avro_generator
            .create_avro_message(PAW_OPPLYSNINGER_TOPIC, opplysninger.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let optional_opplysninger_row = opplysninger::select_by_id(&mut tx, &opplysninger_id)
            .await
            .expect("Kunne ikke hente opplysninger");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_opplysninger_row.is_some());
        let opplysninger_row = optional_opplysninger_row.expect("Ingen opplysninger funnet");
        assert_eq!(opplysninger_row.id, opplysninger_id);
        assert_eq!(opplysninger_row.periode_id, periode_id);
        assert_eq!(
            opplysninger_row.jobbsituasjon,
            vec![Jobbsituasjon::HarBlittSagtOpp.as_ref().to_string()]
        );
    }

    async fn test_process_profilering(context: &TestContext) {
        let identitetsnummer = context.identitetsnummer;
        let periode_id = context.periode_id;
        let opplysninger_id = context.opplysninger_id;
        let profilering_id = context.profilering_id;

        let profilering = create_dummy_profilering(
            identitetsnummer,
            periode_id,
            opplysninger_id,
            profilering_id,
        );
        let message = context
            .avro_generator
            .create_avro_message(PAW_PROFILERING_TOPIC, profilering.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let optional_profilering_row = profilering::select_by_id(&mut tx, &profilering_id)
            .await
            .expect("Kunne ikke hente profilering");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_profilering_row.is_some());
        let profilering_row = optional_profilering_row.expect("Ingen profilering funnet");
        assert_eq!(profilering_row.id, profilering_id);
        assert_eq!(profilering_row.periode_id, periode_id);
        assert_eq!(profilering_row.opplysninger_id, opplysninger_id);
        assert_eq!(
            profilering_row.profilert_til,
            ProfilertTil::AntattGodeMuligheter.as_ref().to_string()
        );
    }

    async fn test_process_egenvurdering(context: &TestContext) {
        let identitetsnummer = context.identitetsnummer;
        let periode_id = context.periode_id;
        let profilering_id = context.profilering_id;
        let egenvurdering_id = context.egenvurdering_id;

        let egenvurdering = create_dummy_egenvurdering(
            identitetsnummer,
            periode_id,
            profilering_id,
            egenvurdering_id,
        );
        let message = context
            .avro_generator
            .create_avro_message(PAW_EGENVURDERING_TOPIC, egenvurdering.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let optional_egenvurdering_row = egenvurdering::select_by_id(&mut tx, &egenvurdering_id)
            .await
            .expect("Kunne ikke hente egenvurdering");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_egenvurdering_row.is_some());
        let egenvurdering_row = optional_egenvurdering_row.expect("Ingen egenvurdering funnet");
        assert_eq!(egenvurdering_row.id, egenvurdering_id);
        assert_eq!(egenvurdering_row.periode_id, periode_id);
        assert_eq!(egenvurdering_row.profilering_id, profilering_id);
        assert_eq!(
            egenvurdering_row.profilert_til,
            ProfilertTil::AntattGodeMuligheter.as_ref().to_string()
        );
        assert_eq!(
            egenvurdering_row.egenvurdert_til,
            ProfilertTil::OppgittHindringer.as_ref().to_string()
        );
    }

    async fn test_process_bekreftelse(context: &TestContext) {
        let identitetsnummer = context.identitetsnummer;
        let periode_id = context.periode_id;
        let bekreftelse_id = context.bekreftelse_id;

        let bekreftelse =
            create_dummy_bekreftelse(identitetsnummer, periode_id, bekreftelse_id, false, true);
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_TOPIC, bekreftelse.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let optional_bekreftelse_row = bekreftelse::select_by_id(&mut tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

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
        assert_eq!(bekreftelse_row.har_jobbet, false);
        assert_eq!(bekreftelse_row.vil_fortsette, true);
    }

    async fn test_process_bekreftelse_paavegneav(context: &TestContext) {
        let periode_id = context.periode_id;

        let paavegneav = create_dummy_paavegneav_start(periode_id, Bekreftelsesloesning::Dagpenger);
        let message = context
            .avro_generator
            .create_avro_message(PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC, paavegneav.clone())
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let paavegneav_rows = bekreftelse_paavegneav::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente paavegneav");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert_eq!(paavegneav_rows.len(), 1);
        let paavegneav_row = paavegneav_rows.first().expect("Ingen periode funnet");
        assert_eq!(paavegneav_row.periode_id, periode_id);
        assert_eq!(
            paavegneav_row.bekreftelsesloesninger,
            vec![Bekreftelsesloesning::Dagpenger.as_ref().to_string()]
        );
    }

    async fn test_process_oppsolgingsperiode(context: &TestContext) {
        let aktor_id = context.aktor_id;
        let identitetsnummer = context.identitetsnummer;
        let oppfolgingsperiode_id = context.oppfolgingsperiode_id;
        let kontor_id = context.kontor_id;

        let oppfolgingsperiode = create_dummy_oppfolgingsperiode_startet(
            oppfolgingsperiode_id,
            aktor_id,
            identitetsnummer,
            kontor_id,
        );
        let message = context
            .json_generator
            .create_json_message(POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC, &oppfolgingsperiode);

        let mut tx = context.start_tx().await;
        let result = context.processor.process_message(&mut tx, &message).await;
        assert!(result.is_ok());
        let optional_kontortilknytning_row =
            kontortilknytning::select_by_id(&mut tx, &oppfolgingsperiode_id)
                .await
                .expect("Kunne ikke hente kontortilknytning");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_kontortilknytning_row.is_some());
        let kontortilknytning_row =
            optional_kontortilknytning_row.expect("Ingen kontortilknytning funnet");
        assert_eq!(kontortilknytning_row.id, oppfolgingsperiode_id);
        assert_eq!(kontortilknytning_row.aktor_id, aktor_id);
        assert_eq!(kontortilknytning_row.identitetsnummer, identitetsnummer);
        assert_eq!(kontortilknytning_row.kontor_id, kontor_id);
        assert_eq!(
            kontortilknytning_row.kontor_type,
            KontorType::Arbeidsoppfolging.as_ref().to_string()
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
            println!("Migrerer databasemodell");
            sqlx::migrate!("./migrations")
                .run(&postgres_guard.pg_pool)
                .await
                .expect("Failed to run migrations");

            TestContext {
                mockito_server,
                mocks,
                pg_pool: postgres_guard.pg_pool,
                json_generator: JsonGenerator,
                avro_generator: AvroGenerator::new(schema_registry_settings.clone()),
                processor: KartleggingMessageProcessor::new(
                    app_config,
                    schema_registry_settings.clone(),
                    key_gen_client,
                    pdl_client,
                )
                .expect("Kunne ikke opprette prosessor"),
                arbeidssoeker_id: 12345,
                aktor_id: "101701234500",
                identitetsnummer: "01017012345",
                periode_id: Uuid::new_v4(),
                opplysninger_id: Uuid::new_v4(),
                profilering_id: Uuid::new_v4(),
                egenvurdering_id: Uuid::new_v4(),
                bekreftelse_id: Uuid::new_v4(),
                oppfolgingsperiode_id: Uuid::new_v4(),
                kontor_id: "1234",
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
        json_generator: JsonGenerator,
        avro_generator: AvroGenerator,
        processor: KartleggingMessageProcessor,
        arbeidssoeker_id: i64,
        aktor_id: &'static str,
        identitetsnummer: &'static str,
        periode_id: Uuid,
        opplysninger_id: Uuid,
        profilering_id: Uuid,
        egenvurdering_id: Uuid,
        bekreftelse_id: Uuid,
        oppfolgingsperiode_id: Uuid,
        kontor_id: &'static str,
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
