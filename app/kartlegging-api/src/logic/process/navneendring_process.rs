use crate::logic::process::PayloadProcessor;
use crate::logic::service::arbeidssoeker_service::ArbeidssoekerService;
use crate::model::dao::arbeidssoeker;
use crate::model::dto::navn::Navn;
use crate::model::error::{DaoError, PayloadProcessorError};
use crate::model::result::ProcessorResult;
use paw_avro_serde::serde::{AvroDeserializer, AvroStringDeserializer};
use paw_kafka::hwm::hwm_message_processor::ProcessorError;
use pdl_hendelser::leesah::{Opplysningstype, Personhendelse};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;

pub struct NavneendringProcessor {
    key_deserializer: AvroStringDeserializer,
    deserializer: AvroDeserializer,
    arbeidssoeker_service: Arc<ArbeidssoekerService>,
}

impl NavneendringProcessor {
    pub fn new(
        schema_registry_settings: SrSettings,
        arbeidssoeker_service: Arc<ArbeidssoekerService>,
    ) -> Self {
        Self {
            key_deserializer: AvroStringDeserializer::new(schema_registry_settings.clone()),
            deserializer: AvroDeserializer::new(schema_registry_settings),
            arbeidssoeker_service,
        }
    }
}

impl PayloadProcessor for NavneendringProcessor {
    #[tracing::instrument(skip_all, fields(topic = %message.topic(), partition = %message.partition(), offset = %message.offset()
    ))]
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<ProcessorResult, ProcessorError> {
        match message.key() {
            None => Err(PayloadProcessorError::no_key_error(message).into()),
            Some(key) => {
                match message.payload() {
                    None => Err(PayloadProcessorError::no_payload_error(message).into()),
                    Some(payload) => {
                        let aktor_id =
                            self.key_deserializer.deserialize(key).await.map_err(|e| {
                                PayloadProcessorError::deserialization_error(message, &e)
                            })?;
                        let hendelse: Personhendelse =
                            self.deserializer.deserialize(payload).await.map_err(|e| {
                                PayloadProcessorError::deserialization_error(message, &e)
                            })?;

                        tracing::debug!(
                            "Mottok {} av type {}:{}",
                            &hendelse,
                            &hendelse.opplysningstype.as_ref(),
                            &hendelse.endringstype.as_ref()
                        );

                        match &hendelse.opplysningstype {
                            Opplysningstype::NavnV1 => {
                                let arbeidssoekere =
                                    arbeidssoeker::select_by_aktor_id(tx, &aktor_id).await?;
                                let count = arbeidssoekere.iter().len();
                                if count == 0 {
                                    tracing::debug!(
                                        "Ignorerer {} av type {}:{} fordi person ikke er arbeidssøker",
                                        &hendelse,
                                        hendelse.opplysningstype.as_ref(),
                                        hendelse.endringstype.as_ref()
                                    );
                                    Ok(ProcessorResult::Continue)
                                } else if count > 1 {
                                    // Mer enn én arbeidssøker funnet for aktør-id
                                    Err(DaoError::multiple_rows(message, "arbeidssoekere", count)
                                        .into())
                                } else {
                                    let arbeidssoeker = arbeidssoekere.first().unwrap();
                                    let optional_navn = self
                                        .arbeidssoeker_service
                                        .hent_navn(message, &arbeidssoeker.identitetsnummer)
                                        .await?;

                                    if optional_navn.is_none() {
                                        tracing::warn!(
                                            "Ignorerer {} av type {}:{} fordi kunne ikke finne navn for person i PDL",
                                            &hendelse,
                                            hendelse.opplysningstype.as_ref(),
                                            hendelse.endringstype.as_ref()
                                        );
                                    } else {
                                        let gjeldende_navn = optional_navn.unwrap();
                                        let lagret_navn = Navn::from(arbeidssoeker);
                                        if lagret_navn == gjeldende_navn {
                                            tracing::debug!(
                                                "Ignorerer {} av type {}:{} fordi navn har ikke endret seg",
                                                &hendelse,
                                                hendelse.opplysningstype.as_ref(),
                                                hendelse.endringstype.as_ref()
                                            );
                                        } else {
                                            tracing::info!(
                                                "Prosesserer {} av type {}:{} for endring av navn",
                                                &hendelse,
                                                hendelse.opplysningstype.as_ref(),
                                                hendelse.endringstype.as_ref()
                                            );
                                            arbeidssoeker::update_navn(
                                                tx,
                                                &aktor_id,
                                                &gjeldende_navn,
                                            )
                                            .await?;
                                        }
                                    }

                                    Ok(ProcessorResult::Continue)
                                }
                            }
                            _ => {
                                tracing::debug!(
                                    "Ignorerer {} av type {}:{}",
                                    &hendelse,
                                    hendelse.opplysningstype.as_ref(),
                                    hendelse.endringstype.as_ref()
                                );
                                Ok(ProcessorResult::Continue)
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::PayloadProcessor;
    use crate::logic::process::navneendring_process::NavneendringProcessor;
    use crate::logic::service::arbeidssoeker_service::ArbeidssoekerService;
    use crate::model::dao::arbeidssoeker;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::error::PayloadProcessorError;
    use crate::model::result::ProcessorResult;
    use chrono::Utc;
    use mockito::{Mock, Server, ServerGuard};
    use paw_key_gen_client::client::PawKeyGenClient;
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use pdl_client::client::PDLClient;
    use pdl_hendelser::leesah::opplysninger::navn::Navn;
    use pdl_hendelser::leesah::{Endringstype, Opplysningstype, Personhendelse};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_mock::schema_definitions::avro_schemas;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock_2;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use test_data_generator::avro::AvroGenerator;
    use token_client_stub::TokenClientStub;
    use uuid::Uuid;

    const PDL_LEESAH_TOPIC: &str = "pdl.leesah-v1";

    #[tokio::test]
    async fn test_process_messages() -> anyhow::Result<()> {
        let context = TestContext::new().await?;
        context.insert_arbeidssoeker_row("Darth", "Vader").await?;

        test_process_messages_with_no_key(&context).await?;
        test_process_messages_with_no_payload(&context).await?;
        test_process_messages_with_irrelevant_type(&context).await?;
        test_process_messages_with_wrong_aktor_id(&context).await?;
        test_process_messages_with_altered_name(&context).await?;
        test_process_messages_with_unaltered_name(&context).await?;

        Ok(())
    }

    async fn test_process_messages_with_no_key(context: &TestContext) -> anyhow::Result<()> {
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context
            .avro_generator
            .create_avro_message(PDL_LEESAH_TOPIC, None, Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.rollback()
            .await
            .expect("Kunne ikke rulle tilbake transaksjon");

        match result {
            Ok(_) => {
                panic!("Test skal feile");
            }
            Err(e) => {
                let error = e
                    .downcast_ref::<PayloadProcessorError>()
                    .expect("Expected PayloadProcessorError");
                assert_eq!(error, &PayloadProcessorError::no_key_error(&message));
                let rows = context.select_arbeidssoeker_row("101701234500").await?;
                assert_eq!(rows.len(), 1);
                let row = rows.get(0).expect("Ingen rad");
                assert_eq!(row.id, 12345);
                assert_eq!(row.aktor_id, "101701234500".to_string());
                assert_eq!(row.identitetsnummer, "01017012345".to_string());
                assert_eq!(row.fornavn, Some("Darth".to_string()));
                assert_eq!(row.mellomnavn, None);
                assert_eq!(row.etternavn, Some("Vader".to_string()));
            }
        }

        Ok(())
    }

    async fn test_process_messages_with_no_payload(context: &TestContext) -> anyhow::Result<()> {
        let key = context
            .avro_generator
            .create_avro_key(PDL_LEESAH_TOPIC, "101701234500".to_string())
            .await;
        let payload: Option<Personhendelse> = None;
        let message = context
            .avro_generator
            .create_avro_message(PDL_LEESAH_TOPIC, Some(key), payload)
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.rollback()
            .await
            .expect("Kunne ikke rulle tilbake transaksjon");

        match result {
            Ok(_) => {
                panic!("Test skal feile");
            }
            Err(e) => {
                let error = e
                    .downcast_ref::<PayloadProcessorError>()
                    .expect("Expected PayloadProcessorError");
                assert_eq!(error, &PayloadProcessorError::no_payload_error(&message));
                let rows = context.select_arbeidssoeker_row("101701234500").await?;
                assert_eq!(rows.len(), 1);
                let row = rows.get(0).expect("Ingen rad");
                assert_eq!(row.id, 12345);
                assert_eq!(row.aktor_id, "101701234500".to_string());
                assert_eq!(row.identitetsnummer, "01017012345".to_string());
                assert_eq!(row.fornavn, Some("Darth".to_string()));
                assert_eq!(row.mellomnavn, None);
                assert_eq!(row.etternavn, Some("Vader".to_string()));
            }
        }

        Ok(())
    }

    async fn test_process_messages_with_irrelevant_type(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let key = context
            .avro_generator
            .create_avro_key(PDL_LEESAH_TOPIC, "101701234500".to_string())
            .await;
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::FoedselsdatoV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context
            .avro_generator
            .create_avro_message(PDL_LEESAH_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        match result {
            Ok(outcome) => {
                assert_eq!(outcome, ProcessorResult::Continue);
                let rows = context.select_arbeidssoeker_row("101701234500").await?;
                assert_eq!(rows.len(), 1);
                let row = rows.get(0).expect("Ingen rad");
                assert_eq!(row.id, 12345);
                assert_eq!(row.aktor_id, "101701234500".to_string());
                assert_eq!(row.identitetsnummer, "01017012345".to_string());
                assert_eq!(row.fornavn, Some("Darth".to_string()));
                assert_eq!(row.mellomnavn, None);
                assert_eq!(row.etternavn, Some("Vader".to_string()));
            }
            Err(_) => {
                panic!("Test skal ikke feile");
            }
        }

        Ok(())
    }

    async fn test_process_messages_with_wrong_aktor_id(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let key = context
            .avro_generator
            .create_avro_key(PDL_LEESAH_TOPIC, "201701234500".to_string())
            .await;
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context
            .avro_generator
            .create_avro_message(PDL_LEESAH_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        match result {
            Ok(outcome) => {
                assert_eq!(outcome, ProcessorResult::Continue);
                let rows_1 = context.select_arbeidssoeker_row("201701234500").await?;
                assert_eq!(rows_1.len(), 0);
                let rows_2 = context.select_arbeidssoeker_row("101701234500").await?;
                assert_eq!(rows_2.len(), 1);
                let row = rows_2.get(0).expect("Ingen rad");
                assert_eq!(row.id, 12345);
                assert_eq!(row.aktor_id, "101701234500".to_string());
                assert_eq!(row.identitetsnummer, "01017012345".to_string());
                assert_eq!(row.fornavn, Some("Darth".to_string()));
                assert_eq!(row.mellomnavn, None);
                assert_eq!(row.etternavn, Some("Vader".to_string()));
            }
            Err(_) => {
                panic!("Test skal ikke feile");
            }
        }

        Ok(())
    }

    async fn test_process_messages_with_altered_name(context: &TestContext) -> anyhow::Result<()> {
        let key = context
            .avro_generator
            .create_avro_key(PDL_LEESAH_TOPIC, "101701234500".to_string())
            .await;
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context
            .avro_generator
            .create_avro_message(PDL_LEESAH_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        match result {
            Ok(outcome) => {
                assert_eq!(outcome, ProcessorResult::Continue);
                let rows = context.select_arbeidssoeker_row("101701234500").await?;
                assert_eq!(rows.len(), 1);
                let row = rows.get(0).expect("Ingen rad");
                assert_eq!(row.id, 12345);
                assert_eq!(row.aktor_id, "101701234500".to_string());
                assert_eq!(row.identitetsnummer, "01017012345".to_string());
                assert_eq!(row.fornavn, Some("Anakin".to_string()));
                assert_eq!(row.mellomnavn, None);
                assert_eq!(row.etternavn, Some("Skywalker".to_string()));
            }
            Err(_) => {
                panic!("Test skal ikke feile");
            }
        }

        Ok(())
    }

    async fn test_process_messages_with_unaltered_name(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let key = context
            .avro_generator
            .create_avro_key(PDL_LEESAH_TOPIC, "101701234500".to_string())
            .await;
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context
            .avro_generator
            .create_avro_message(PDL_LEESAH_TOPIC, Some(key), Some(payload))
            .await;

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        match result {
            Ok(outcome) => {
                assert_eq!(outcome, ProcessorResult::Continue);
                let rows = context.select_arbeidssoeker_row("101701234500").await?;
                assert_eq!(rows.len(), 1);
                let row = rows.get(0).expect("Ingen rad");
                assert_eq!(row.id, 12345);
                assert_eq!(row.aktor_id, "101701234500".to_string());
                assert_eq!(row.identitetsnummer, "01017012345".to_string());
                assert_eq!(row.fornavn, Some("Anakin".to_string()));
                assert_eq!(row.mellomnavn, None);
                assert_eq!(row.etternavn, Some("Skywalker".to_string()));
            }
            Err(_) => {
                panic!("Test skal ikke feile");
            }
        }

        Ok(())
    }

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        avro_generator: AvroGenerator,
        processor: NavneendringProcessor,
    }

    impl TestContext {
        async fn new() -> anyhow::Result<Self> {
            let context = {
                let mut mockito_server = Server::new_async().await;

                let schemas = avro_schemas();
                let schema_registry_guard =
                    create_schema_registry_mock_2(&mut mockito_server, schemas)
                        .await
                        .expect("Failed to create schema registry mock");
                let schema_registry_settings = schema_registry_guard.schema_registry_settings;

                let pdl_mock_responses = default_pdl_mock_responses();
                let pdl_mock_guard = init_pdl_mock(&mut mockito_server, pdl_mock_responses)
                    .await
                    .expect("Kunne ikke initialisere PDL mock server");

                let mut schema_registry_mocks = schema_registry_guard.mocks;
                let mut mocks = pdl_mock_guard.mocks;
                mocks.append(&mut schema_registry_mocks);

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

                let arbeidssoeker_service = ArbeidssoekerService::new(key_gen_client, pdl_client);

                let postgres_guard = setup_postgres_container()
                    .await
                    .expect("Failed to start Postgres container");
                println!("Migrerer databasemodell");
                sqlx::migrate!("./migrations")
                    .run(&postgres_guard.pg_pool)
                    .await
                    .expect("Failed to run migrations");

                Self {
                    mockito_server,
                    mocks,
                    pg_pool: postgres_guard.pg_pool,
                    avro_generator: AvroGenerator::new(schema_registry_settings.clone()),
                    processor: NavneendringProcessor::new(
                        schema_registry_settings.clone(),
                        arbeidssoeker_service,
                    ),
                }
            };

            Ok(context)
        }

        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }

        async fn select_arbeidssoeker_row(
            &self,
            aktor_id: &str,
        ) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
            let mut tx = self.start_tx().await;
            let rows = arbeidssoeker::select_by_aktor_id(&mut tx, aktor_id).await?;
            tx.commit().await.expect("Kunne ikke commit transaksjon");
            Ok(rows)
        }

        async fn insert_arbeidssoeker_row(
            &self,
            fornavn: &str,
            etternavn: &str,
        ) -> anyhow::Result<ArbeidssoekerRow> {
            let row = self.create_arbeidssoeker_row(fornavn, etternavn);
            let mut tx = self.start_tx().await;
            arbeidssoeker::insert(&mut tx, &row).await?;
            tx.commit().await.expect("Kunne ikke commit transaksjon");
            Ok(row)
        }

        fn create_hendelse(
            &self,
            personidenter: Vec<String>,
            opplysningstype: Opplysningstype,
            endringstype: Endringstype,
            navn: Option<Navn>,
        ) -> Personhendelse {
            Personhendelse {
                hendelse_id: Uuid::new_v4().to_string(),
                personidenter,
                master: "PDL".to_string(),
                opprettet: Utc::now(),
                opplysningstype,
                endringstype,
                tidligere_hendelse_id: None,
                adressebeskyttelse: None,
                delt_bosted: None,
                doedfoedt_barn: None,
                doedsfall: None,
                falsk_identitet: None,
                foedsel: None,
                foedselsdato: None,
                forelder_barn_relasjon: None,
                familierelasjon: None,
                sivilstand: None,
                vergemaal_eller_fremtidsfullmakt: None,
                utflytting_fra_norge: None,
                innflytting_til_norge: None,
                folkeregisteridentifikator: None,
                navn,
                sikkerhetstiltak: None,
                statsborgerskap: None,
                telefonnummer: None,
                kontaktadresse: None,
                bostedsadresse: None,
            }
        }

        fn create_arbeidssoeker_row(&self, fornavn: &str, etternavn: &str) -> ArbeidssoekerRow {
            ArbeidssoekerRow {
                id: 12345,
                aktor_id: "101701234500".to_string(),
                identitetsnummer: "01017012345".to_string(),
                fornavn: Some(fornavn.to_string()),
                mellomnavn: None,
                etternavn: Some(etternavn.to_string()),
            }
        }
    }
}
