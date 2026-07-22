use crate::logic::process::PayloadProcessor;
use crate::model::dao::kontortilknytning;
use crate::model::dao::kontortilknytning::KontortilknytningRow;
use crate::model::dto::kontortilknytning::KontorType;
use crate::model::error::{DaoError, PayloadProcessorError};
use dab_oppfolgingperioder::oppfolgingsperiode::{
    Oppfolgingsperiode, OppfolgingsperiodeAvsluttet, OppfolgingsperiodeEndret,
};
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use sqlx::{Postgres, Transaction};

pub struct OppfolgingsperiodeProcessor;

impl OppfolgingsperiodeProcessor {
    pub fn new() -> Self {
        Self {}
    }

    async fn upsert_kontortilknytning<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &OwnedMessage,
        data: &'a OppfolgingsperiodeEndret,
    ) -> anyhow::Result<u64> {
        let row = KontortilknytningRow::new(
            data.id.clone(),
            data.aktor_id.clone(),
            data.ident.clone(),
            data.kontor.kontor_id.clone(),
            data.kontor.kontor_navn.clone(),
            KontorType::Arbeidsoppfolging.as_ref().to_string(), // Akkurat nå vil alle kontortilknytninger være av type Arbeidsoppfolging. Feltet åpner for å kunne ta imot andre typer tilknytninger i fremtiden
            data.start_tidspunkt.clone(),
        );
        let count = kontortilknytning::count_by_id(tx, &data.id).await?;
        if count > 1 {
            Err(DaoError::multiple_rows(message, "bekreftelse_paavegneav", count as usize).into())
        } else if count == 1 {
            kontortilknytning::update(tx, &row).await
        } else {
            kontortilknytning::insert(tx, &row).await
        }
    }

    async fn delete_kontortilknytning<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        data: &'a OppfolgingsperiodeAvsluttet,
    ) -> anyhow::Result<u64> {
        kontortilknytning::delete(tx, &data.id).await
    }
}

impl PayloadProcessor for OppfolgingsperiodeProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<(), ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: Oppfolgingsperiode = serde_json::from_slice(payload)
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok Oppfolgingsperiode-hendelse");

                match hendelse {
                    Oppfolgingsperiode::Startet(data) => {
                        self.upsert_kontortilknytning(tx, &message, &data).await?
                    }
                    Oppfolgingsperiode::Endret(data) => {
                        self.upsert_kontortilknytning(tx, &message, &data).await?
                    }
                    Oppfolgingsperiode::Avsluttet(data) => {
                        self.delete_kontortilknytning(tx, &data).await?
                    }
                };

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::oppfolgingsperiode_process::OppfolgingsperiodeProcessor;
    use crate::logic::process::PayloadProcessor;
    use crate::model::dao::kontortilknytning;
    use crate::model::dto::kontortilknytning::KontorType;
    use dab_oppfolgingperioder::oppfolgingsperiode::{
        Oppfolgingsperiode, POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC,
    };
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::{PgPool, Postgres, Transaction};
    use test_data_generator::dab_oppfolgingsperiode::{
        create_dummy_oppfolgingsperiode_avsluttet, create_dummy_oppfolgingsperiode_endret,
        create_dummy_oppfolgingsperiode_startet,
    };
    use test_data_generator::json::JsonGenerator;
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        test_process_oppfolgingsperiode_startet(context).await;
        test_process_oppfolgingsperiode_endret(context).await;
        test_process_oppfolgingsperiode_avsluttet(context).await;
    }

    async fn test_process_oppfolgingsperiode_startet(context: &TestContext) {
        let aktor_id = context.aktor_id;
        let identitetsnummer = context.identitetsnummer;
        let oppfolgingsperiode_id = context.oppfolgingsperiode_id;
        let kontor_id_1 = context.kontor_id_1;

        let oppfolgingsperiode = create_dummy_oppfolgingsperiode_startet(
            oppfolgingsperiode_id,
            aktor_id,
            identitetsnummer,
            kontor_id_1,
        );
        let message = context
            .json_generator
            .create_json_message(POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC, &oppfolgingsperiode);

        let mut tx_1 = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx_1, &message).await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let mut tx_2 = context.start_tx().await;
        let optional_kontortilknytning_row =
            kontortilknytning::select_by_id(&mut tx_2, &oppfolgingsperiode_id)
                .await
                .expect("Kunne ikke hente kontortilknytning");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_kontortilknytning_row.is_some());
        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(hendelse) => {
                let kontortilknytning_row =
                    optional_kontortilknytning_row.expect("Ingen kontortilknytning funnet");
                assert_eq!(kontortilknytning_row.id, hendelse.id);
                assert_eq!(kontortilknytning_row.aktor_id, hendelse.aktor_id);
                assert_eq!(kontortilknytning_row.kontor_id, hendelse.kontor.kontor_id);
                assert_eq!(
                    kontortilknytning_row.kontor_type,
                    KontorType::Arbeidsoppfolging.as_ref().to_string()
                );
                assert_eq!(
                    kontortilknytning_row.kontor_navn,
                    hendelse.kontor.kontor_navn
                );
            }
            Oppfolgingsperiode::Endret(_) => {
                panic!("Uventet hendelsetype")
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Uventet hendelsetype")
            }
        }
    }

    async fn test_process_oppfolgingsperiode_endret(context: &TestContext) {
        let aktor_id = context.aktor_id;
        let identitetsnummer = context.identitetsnummer;
        let oppfolgingsperiode_id = context.oppfolgingsperiode_id;
        let kontor_id_2 = context.kontor_id_2;

        let oppfolgingsperiode = create_dummy_oppfolgingsperiode_endret(
            oppfolgingsperiode_id,
            aktor_id,
            identitetsnummer,
            kontor_id_2,
        );
        let message = context
            .json_generator
            .create_json_message(POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC, &oppfolgingsperiode);

        let mut tx_1 = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx_1, &message).await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result.is_ok());

        let mut tx_2 = context.start_tx().await;
        let optional_kontortilknytning_row =
            kontortilknytning::select_by_id(&mut tx_2, &oppfolgingsperiode_id)
                .await
                .expect("Kunne ikke hente kontortilknytning");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_kontortilknytning_row.is_some());
        match oppfolgingsperiode {
            Oppfolgingsperiode::Startet(_) => {
                panic!("Uventet hendelsetype")
            }
            Oppfolgingsperiode::Endret(hendelse) => {
                let kontortilknytning_row =
                    optional_kontortilknytning_row.expect("Ingen kontortilknytning funnet");
                assert_eq!(kontortilknytning_row.id, hendelse.id);
                assert_eq!(kontortilknytning_row.aktor_id, hendelse.aktor_id);
                assert_eq!(kontortilknytning_row.kontor_id, hendelse.kontor.kontor_id);
                assert_eq!(
                    kontortilknytning_row.kontor_type,
                    KontorType::Arbeidsoppfolging.as_ref().to_string()
                );
                assert_eq!(
                    kontortilknytning_row.kontor_navn,
                    hendelse.kontor.kontor_navn
                );
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Uventet hendelsetype")
            }
        }
    }

    async fn test_process_oppfolgingsperiode_avsluttet(context: &TestContext) {
        let aktor_id = context.aktor_id;
        let identitetsnummer = context.identitetsnummer;
        let oppfolgingsperiode_id = context.oppfolgingsperiode_id;

        let oppfolgingsperiode = create_dummy_oppfolgingsperiode_avsluttet(
            oppfolgingsperiode_id,
            aktor_id,
            identitetsnummer,
        );
        let message = context
            .json_generator
            .create_json_message(POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC, &oppfolgingsperiode);

        let mut tx_1 = context.start_tx().await;
        let result = context.processor.process_payload(&mut tx_1, &message).await;
        tx_1.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result.is_ok());

        let mut tx_2 = context.start_tx().await;
        let optional_kontortilknytning_row =
            kontortilknytning::select_by_id(&mut tx_2, &oppfolgingsperiode_id)
                .await
                .expect("Kunne ikke hente oppfolgingsperiode");
        tx_2.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_kontortilknytning_row.is_none());
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> &'static TestContext {
        INIT.get_or_init(|| async {
            let postgres_guard = setup_postgres_container(5432)
                .await
                .expect("Failed to start Postgres container");
            sqlx::migrate!("./migrations")
                .run(&postgres_guard.pg_pool)
                .await
                .expect("Failed to run migrations");
            let processor = OppfolgingsperiodeProcessor::new();
            TestContext {
                pg_pool: postgres_guard.pg_pool,
                json_generator: JsonGenerator,
                processor,
                aktor_id: "101701234500",
                identitetsnummer: "01017012345",
                oppfolgingsperiode_id: Uuid::new_v4(),
                kontor_id_1: "1337",
                kontor_id_2: "1234",
            }
        })
        .await
    }

    struct TestContext {
        pg_pool: PgPool,
        json_generator: JsonGenerator,
        processor: OppfolgingsperiodeProcessor,
        aktor_id: &'static str,
        identitetsnummer: &'static str,
        oppfolgingsperiode_id: Uuid,
        kontor_id_1: &'static str,
        kontor_id_2: &'static str,
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
