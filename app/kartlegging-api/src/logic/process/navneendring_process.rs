use crate::logic::process::PayloadProcessor;
use crate::logic::service::arbeidssoeker_service::ArbeidssoekerService;
use crate::model::dao::arbeidssoeker;
use crate::model::dto::navn::Navn;
use crate::model::error::{DaoError, PayloadProcessorError};
use crate::model::result::ProcessorResult;
use paw_kafka::hwm::hwm_message_processor::ProcessorError;
use pdl_hendelser::leesah::{Opplysningstype, PersonhendelseDeserializer};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;

pub struct NavneendringProcessor {
    deserializer: PersonhendelseDeserializer,
    arbeidssoeker_service: Arc<ArbeidssoekerService>,
}

impl NavneendringProcessor {
    pub fn new(arbeidssoeker_service: Arc<ArbeidssoekerService>) -> Self {
        Self {
            deserializer: PersonhendelseDeserializer::new().unwrap(),
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
                        let hendelse = self.deserializer.deserialize(payload)?;

                        tracing::debug!(
                            "Mottok {} av type {}:{}",
                            &hendelse,
                            &hendelse.opplysningstype.as_ref(),
                            &hendelse.endringstype.as_ref()
                        );

                        match &hendelse.opplysningstype {
                            Opplysningstype::NavnV1 => {
                                let aktor_id = String::from_utf8(key.to_vec())?;

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
    use pdl_hendelser::leesah::{
        Endringstype, Opplysningstype, Personhendelse, PersonhendelseSerializer,
    };
    use postgres_testcontainer::postgres::setup_postgres_container;
    use rdkafka::Timestamp;
    use rdkafka::message::OwnedMessage;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use token_client_stub::TokenClientStub;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_process_messages() -> anyhow::Result<()> {
        let context = &init().await?;
        context
                .insert_arbeidssoeker_row("Darth", "Vader")
                .await?;

        test_process_messages_with_no_key(context).await?;
        test_process_messages_with_no_payload(context).await?;
        test_process_messages_with_irrelevant_type(context).await?;
        test_process_messages_with_wrong_aktor_id(context).await?;
        test_process_messages_with_altered_name(context).await?;
        test_process_messages_with_unaltered_name(context).await?;

        Ok(())
    }

    async fn test_process_messages_with_no_key(context: &TestContext) -> anyhow::Result<()> {
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context.create_avro_message(None, Some(&payload));

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

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
        let message = context.create_avro_message(Some("101701234500"), None);

        let mut tx = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx, &message).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

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
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::FoedselsdatoV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context.create_avro_message(Some("101701234500"), Some(&payload));

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
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context.create_avro_message(Some("201701234500"), Some(&payload));

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
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context.create_avro_message(Some("101701234500"), Some(&payload));

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
        let payload = context.create_hendelse(
            vec![],
            Opplysningstype::NavnV1,
            Endringstype::Opprettet,
            None,
        );
        let message = context.create_avro_message(Some("101701234500"), Some(&payload));

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

    async fn init() -> anyhow::Result<TestContext> {
        let mut mockito_server = Server::new_async().await;

        let pdl_mock_responses = default_pdl_mock_responses();
        let pdl_mock_guard = init_pdl_mock(&mut mockito_server, pdl_mock_responses)
            .await
            .expect("Kunne ikke initialisere PDL mock server");

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
        sqlx::migrate!("./migrations")
            .run(&postgres_guard.pg_pool)
            .await
            .expect("Failed to run migrations");

        let context = TestContext {
            mockito_server,
            mocks: pdl_mock_guard.mocks,
            pg_pool: postgres_guard.pg_pool,
            processor: NavneendringProcessor::new(arbeidssoeker_service),
            serializer: PersonhendelseSerializer::new()?,
        };

        Ok(context)
    }

    struct TestContext {
        #[allow(unused)]
        mockito_server: ServerGuard,
        #[allow(unused)]
        mocks: Vec<Mock>,
        pg_pool: PgPool,
        processor: NavneendringProcessor,
        serializer: PersonhendelseSerializer,
    }

    impl TestContext {
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

        fn create_avro_key(&self, key: &str) -> Vec<u8> {
            key.as_bytes().to_vec()
        }

        fn create_avro_payload(&self, payload: &Personhendelse) -> Vec<u8> {
            self.serializer
                .serialize(payload)
                .expect("Kunne ikke serialisere melding")
        }

        fn create_avro_message(
            &self,
            key: Option<&str>,
            payload: Option<&Personhendelse>,
        ) -> OwnedMessage {
            OwnedMessage::new(
                payload.map(|p| self.create_avro_payload(p)),
                key.map(|k| self.create_avro_key(k)),
                "dummy-topic".to_string(),
                Timestamp::now(),
                0,
                0,
                None,
            )
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
