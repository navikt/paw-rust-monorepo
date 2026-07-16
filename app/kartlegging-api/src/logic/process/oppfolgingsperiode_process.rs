use crate::logic::mutation::kontortilknytning_mutation;
use dab_oppfolgingperioder::oppfolgingsperiode::Oppfolgingsperiode;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use sqlx::{Postgres, Transaction};

pub struct OppfolgingsperiodeProcessor;

impl OppfolgingsperiodeProcessor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: Oppfolgingsperiode = serde_json::from_slice(payload).map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;

        tracing::info!("Mottok hendelse: {:?}", &hendelse);

        kontortilknytning_mutation::lagre_hendelse(tx, &hendelse).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::logic::process::oppfolgingsperiode_process::OppfolgingsperiodeProcessor;
    use crate::model::dao::kontortilknytning;
    use crate::model::dto::kontortilknytning::KontorType;
    use dab_oppfolgingperioder::oppfolgingsperiode::Oppfolgingsperiode;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::{PgPool, Postgres, Transaction};
    use test_data_generator::dab_oppfolgingsperiode::{
        create_dummy_oppfolgingsperiode_avsluttet, create_dummy_oppfolgingsperiode_endret,
        create_dummy_oppfolgingsperiode_startet,
    };
    use tokio::sync::OnceCell;
    use uuid::Uuid;

    struct TestContext {
        pg_pool: PgPool,
        processor: OppfolgingsperiodeProcessor,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }
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
                processor,
            }
        })
        .await
    }

    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;

        let oppfolgingsperiode_id = Uuid::new_v4();
        let oppfolgingsperiode_1 = create_dummy_oppfolgingsperiode_startet(
            oppfolgingsperiode_id,
            "101701234500",
            "01017012345",
            "1337",
        );
        let payload_1 = serde_json::to_vec(&oppfolgingsperiode_1)
            .expect("Kunne ikke serialisere oppfolgingsperiode");

        let mut tx = context.start_tx().await;
        let result_1 = context.processor.process_payload(&mut tx, &payload_1).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_1.is_ok());

        let mut tx = context.start_tx().await;
        let optional_row_1 = kontortilknytning::select_by_id(&mut tx, &oppfolgingsperiode_id)
            .await
            .expect("Kunne ikke hente oppfolgingsperiode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_row_1.is_some());
        match oppfolgingsperiode_1 {
            Oppfolgingsperiode::Startet(hendelse) => {
                let row = optional_row_1.expect("Ingen oppfolgingsperiode funnet");
                assert_eq!(row.id, hendelse.id);
                assert_eq!(row.aktor_id, hendelse.aktor_id);
                assert_eq!(row.kontor_id, hendelse.kontor.kontor_id);
                assert_eq!(
                    row.kontor_type,
                    KontorType::Arbeidsoppfolging.as_ref().to_string()
                );
                assert_eq!(row.kontor_navn, hendelse.kontor.kontor_navn);
            }
            Oppfolgingsperiode::Endret(_) => {
                panic!("Uventet hendelsetype")
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Uventet hendelsetype")
            }
        }

        let oppfolgingsperiode_2 = create_dummy_oppfolgingsperiode_endret(
            oppfolgingsperiode_id,
            "101701234500",
            "01017012345",
            "1234",
        );
        let payload_2 = serde_json::to_vec(&oppfolgingsperiode_2)
            .expect("Kunne ikke serialisere oppfolgingsperiode");

        let mut tx = context.start_tx().await;
        let result_2 = context.processor.process_payload(&mut tx, &payload_2).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_2.is_ok());

        let mut tx = context.start_tx().await;
        let optional_row_2 = kontortilknytning::select_by_id(&mut tx, &oppfolgingsperiode_id)
            .await
            .expect("Kunne ikke hente oppfolgingsperiode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_row_2.is_some());
        match oppfolgingsperiode_2 {
            Oppfolgingsperiode::Startet(_) => {
                panic!("Uventet hendelsetype")
            }
            Oppfolgingsperiode::Endret(hendelse) => {
                let row = optional_row_2.expect("Ingen oppfolgingsperiode funnet");
                assert_eq!(row.id, hendelse.id);
                assert_eq!(row.aktor_id, hendelse.aktor_id);
                assert_eq!(row.kontor_id, hendelse.kontor.kontor_id);
                assert_eq!(
                    row.kontor_type,
                    KontorType::Arbeidsoppfolging.as_ref().to_string()
                );
                assert_eq!(row.kontor_navn, hendelse.kontor.kontor_navn);
            }
            Oppfolgingsperiode::Avsluttet(_) => {
                panic!("Uventet hendelsetype")
            }
        }

        let oppfolgingsperiode_3 = create_dummy_oppfolgingsperiode_avsluttet(
            oppfolgingsperiode_id,
            "101701234500",
            "01017012345",
        );
        let payload_3 = serde_json::to_vec(&oppfolgingsperiode_3)
            .expect("Kunne ikke serialisere oppfolgingsperiode");

        let mut tx = context.start_tx().await;
        let result_3 = context.processor.process_payload(&mut tx, &payload_3).await;
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(result_3.is_ok());

        let mut tx = context.start_tx().await;
        let optional_row_3 = kontortilknytning::select_by_id(&mut tx, &oppfolgingsperiode_id)
            .await
            .expect("Kunne ikke hente oppfolgingsperiode");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(optional_row_3.is_none());
    }
}
