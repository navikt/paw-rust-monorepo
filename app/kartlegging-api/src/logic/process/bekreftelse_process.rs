use crate::config::AppConfig;
use crate::logic::process::PayloadProcessor;
use crate::logic::process::kartlegging_process::utled_arbeidsledighet_for_bekreftelse;
use crate::model::dao::bekreftelse::BekreftelseRow;
use crate::model::dao::{bekreftelse, kartlegging};
use crate::model::error::{DaoError, PayloadProcessorError};
use crate::model::result::ProcessorResult;
use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
use eksterne_hendelser::serde::AvroDeserializer;
use eksterne_hendelser::vo::metadata::Metadata;
use paw_kafka::hwm::hwm_message_processor::ProcessorError;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;

pub struct BekreftelseProcessor {
    pub app_config: Arc<AppConfig>,
    pub deserializer: AvroDeserializer,
    synced_topics: Vec<String>,
}

impl BekreftelseProcessor {
    pub fn new(app_config: Arc<AppConfig>, schema_registry_settings: SrSettings) -> Self {
        Self {
            app_config: app_config.clone(),
            deserializer: AvroDeserializer::new(schema_registry_settings),
            synced_topics: app_config.kafka.synced_topics_as_vec(),
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
            hendelse.svar.sendt_inn_av.tidspunkt().to_owned(),
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

    async fn lagre_kartlegging<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &OwnedMessage,
        hendelse: &'a Bekreftelse,
    ) -> anyhow::Result<ProcessorResult, ProcessorError> {
        let kartlegging_rows = kartlegging::select_by_periode_id(tx, &hendelse.periode_id).await?;
        let count = kartlegging_rows.len();
        if count > 1 {
            Err(DaoError::multiple_rows(message, "kartlegginger", count).into())
        } else if count == 1 {
            let kartlegging_row = kartlegging_rows
                .first()
                .ok_or_else(|| DaoError::no_rows(message, "kartlegginger"))?;
            // Hent bekreftelser for periode-id
            let bekreftelse_rows =
                bekreftelse::select_by_periode_id(tx, &kartlegging_row.periode_id).await?;
            let siste_bekreftelse_row = bekreftelse_rows.iter().max_by_key(|&row| row.gjelder_til);

            if hendelse.svar.gjelder_til <= kartlegging_row.arbeidssoeker_fra {
                tracing::warn!("Bekreftelseperiode er tidligere enn arbeidssøkerperiode");
            } else if siste_bekreftelse_row.is_some()
                && hendelse.svar.gjelder_til < siste_bekreftelse_row.unwrap().gjelder_til
            {
                tracing::warn!(
                    "Bekreftelseperiode er tidligere enn nyeste eksisterende bekreftelseperiode"
                );
            } else {
                let arbeidsledig_fra = utled_arbeidsledighet_for_bekreftelse(
                    tx,
                    &hendelse,
                    &kartlegging_row,
                    &bekreftelse_rows,
                    self.app_config.periode_gap_grense_for_ledighet,
                )
                .await?;

                kartlegging::update(
                    tx,
                    &hendelse.periode_id,
                    &kartlegging_row.arbeidssoeker_til,
                    &arbeidsledig_fra,
                )
                .await?;
            }

            Ok(ProcessorResult::Continue)
        } else {
            tracing::debug!("Fant ingen kartlegginger for periode-id ennå, avventer periode");
            Ok(ProcessorResult::Pause {
                synced_topics: self.synced_topics.clone(),
            })
        }
    }
}

impl PayloadProcessor for BekreftelseProcessor {
    #[tracing::instrument(skip_all, fields(topic = %message.topic(), partition = %message.partition(), offset = %message.offset()))]
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<ProcessorResult, ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: Bekreftelse = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok {}-hendelse", &hendelse);

                // Lagre bekreftelse
                self.lagre_bekreftelse(tx, &message, &hendelse).await?;

                // Lagre kartlegging
                self.lagre_kartlegging(tx, &message, &hendelse).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::read_app_config;
    use crate::logic::process::PayloadProcessor;
    use crate::logic::process::bekreftelse_process::BekreftelseProcessor;
    use crate::logic::process::kartlegging_process::utled_arbeidsledighet_for_bekreftelse;
    use crate::logic::process::periode_process::PeriodeProcessor;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::dao::kartlegging::KartleggingRow;
    use crate::model::dao::{arbeidssoeker, bekreftelse, kartlegging, periode};
    use crate::model::result::ProcessorResult;
    use chrono::{Duration, TimeZone, Utc};
    use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use eksterne_hendelser::bekreftelse::vo::svar::Svar;
    use mockito::{Mock, Server, ServerGuard};
    use paw_kafka_key_gen_api_mock::{
        default_kafka_key_gen_mock_responses, init_kafka_key_gen_api_mocks,
    };
    use paw_key_gen_client::client::PawKeyGenClient;
    use pdl_api_mock::{default_pdl_mock_responses, init_pdl_mock};
    use pdl_client::client::PDLClient;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
    use sqlx::{PgPool, Postgres, Transaction};
    use std::sync::Arc;
    use test_data_generator::avro::AvroGenerator;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_bekreftelse, create_dummy_bekreftelse_metadata, create_dummy_start_periode,
    };
    use token_client_stub::TokenClientStub;
    use tracing_test::traced_test;
    use uuid::Uuid;

    #[traced_test]
    #[tokio::test]
    async fn test_process_messages() {
        let context = &init().await;

        test_process_periode(context).await;
        test_process_bekreftelse_1_med_periode(context).await;
        test_process_bekreftelse_for_perioden_beholder_ledighet(context).await;
        test_process_bekreftelse_2_med_periode(context).await;
        test_process_bekreftelse_3_uten_periode(context).await;
        test_process_bekreftelse_4_kaldstart_periode_ankommer_senere(context).await;
        test_utled_arbeidsledighet_fra_bekreftelse_tidligere_kartlegging(context).await;
    }

    async fn test_process_periode(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer = context.identitetsnummer_1;
        let periode_id = context.periode_id_1;

        let periode = create_dummy_start_periode(identitetsnummer, periode_id, None);
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssokerperioder-v1", periode)
            .await;

        let mut tx = context.start_tx().await;
        let result = context
            .periode_processor
            .process_payload(&mut tx, &message)
            .await;
        eprintln!("Result: {:?}", result);
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

        let bekreftelse = create_dummy_bekreftelse(
            identitetsnummer,
            periode_id,
            bekreftelse_id,
            None,
            None,
            false,
            true,
        );
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssoker-bekreftelse-v1", bekreftelse)
            .await;

        let mut tx = context.start_tx().await;
        let result = context
            .bekreftelse_processor
            .process_payload(&mut tx, &message)
            .await;
        assert!(result.is_ok());
        let optional_bekreftelse_row = bekreftelse::select_by_id(&mut tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
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

    async fn test_process_bekreftelse_for_perioden_beholder_ledighet(context: &TestContext) {
        let periode_id = context.periode_id_1;
        let bekreftelse_id = Uuid::new_v4();
        let mut tx = context.start_tx().await;
        let kartlegging_row = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging")
            .pop()
            .expect("Ingen kartlegging funnet");
        let arbeidsledig_fra = kartlegging_row
            .arbeidsledig_fra
            .expect("Forventet arbeidsledig_fra fra første bekreftelse");
        let hendelse = Bekreftelse {
            id: bekreftelse_id,
            periode_id,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
            svar: Svar {
                sendt_inn_av: create_dummy_bekreftelse_metadata(context.identitetsnummer_1, None),
                gjelder_fra: kartlegging_row.arbeidssoeker_fra - Duration::days(14),
                gjelder_til: kartlegging_row.arbeidssoeker_fra,
                har_jobbet_i_denne_perioden: false,
                vil_fortsette_som_arbeidssoeker: true,
            },
        };
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssoker-bekreftelse-v1", hendelse)
            .await;

        context
            .bekreftelse_processor
            .process_payload(&mut tx, &message)
            .await
            .expect("Bekreftelse før perioden skal ignoreres");

        let kartlegging_row = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging")
            .pop()
            .expect("Ingen kartlegging funnet");
        let bekreftelse_row = bekreftelse::select_by_id(&mut tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        tx.commit().await.expect("Kunne ikke commit transaksjon");

        assert!(bekreftelse_row.is_some());
        assert_eq!(kartlegging_row.arbeidsledig_fra, Some(arbeidsledig_fra));
    }

    async fn test_process_bekreftelse_2_med_periode(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer = context.identitetsnummer_1;
        let periode_id = context.periode_id_1;
        let bekreftelse_id = context.bekreftelse_id_2;

        let bekreftelse = create_dummy_bekreftelse(
            identitetsnummer,
            periode_id,
            bekreftelse_id,
            None,
            None,
            true,
            true,
        );
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssoker-bekreftelse-v1", bekreftelse)
            .await;

        let mut tx = context.start_tx().await;
        let result = context
            .bekreftelse_processor
            .process_payload(&mut tx, &message)
            .await;
        assert!(result.is_ok());
        let optional_bekreftelse_row = bekreftelse::select_by_id(&mut tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
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

        let bekreftelse = create_dummy_bekreftelse(
            identitetsnummer,
            periode_id,
            bekreftelse_id,
            None,
            None,
            false,
            true,
        );
        let message = context
            .avro_generator
            .create_avro_message("paw.arbeidssoker-bekreftelse-v1", bekreftelse)
            .await;

        let mut tx = context.start_tx().await;
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        let optional_periode_row = periode::select_by_id(&mut tx, &periode_id)
            .await
            .expect("Kunne ikke hente periode");
        assert!(kartlegging_rows.is_empty());
        assert!(optional_periode_row.is_none());
        let result = context
            .bekreftelse_processor
            .process_payload(&mut tx, &message)
            .await;

        let outcome = result.expect("Forventet Ok(ProcessingOutcome::Pause)");
        match outcome {
            ProcessorResult::Pause { synced_topics } => {
                assert_eq!(
                    synced_topics,
                    vec!["paw.arbeidssokerperioder-v1".to_string()]
                );
            }
            ProcessorResult::Continue => panic!("Uventet variant"),
        }

        tx.rollback().await.expect("Kunne ikke rulle tilbake");
        let mut verify_tx = context.start_tx().await;
        let optional_row = bekreftelse::select_by_id(&mut verify_tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        verify_tx
            .commit()
            .await
            .expect("Kunne ikke commit transaksjon");
        assert!(
            optional_row.is_none(),
            "Bekreftelse skal ikke være persistert før periode finnes"
        );
    }

    /// Integrasjonstest for kaldstart-scenarioet beskrevet i kafka-synchronization-plan.md:
    /// en bekreftelse-melding ankommer *før* perioden den refererer til er prosessert. Verifiserer
    /// hele forløpet på tvers av `BekreftelseProcessor` og `PeriodeProcessor` (uten den ekte
    /// konsumentløkken/pause-controlleren, som er dekket av rene enhetstester i
    /// `kafka/consumer.rs`): første forsøk skal returnere `ProcessingOutcome::Pause` og ikke
    /// persistere noe, deretter skal periode-prosessering fullføre normalt, og til slutt skal en
    /// re-levering av bekreftelsen (slik konsumentløkken ville gjort etter pause+seek+resume)
    /// fullføre og koble bekreftelsen til riktig `kartlegginger`-rad.
    async fn test_process_bekreftelse_4_kaldstart_periode_ankommer_senere(context: &TestContext) {
        let identitetsnummer = context.identitetsnummer_1;
        let periode_id = context.periode_id_4;
        let bekreftelse_id = context.bekreftelse_id_4;

        let bekreftelse = create_dummy_bekreftelse(
            identitetsnummer,
            periode_id,
            bekreftelse_id,
            None,
            None,
            false,
            true,
        );
        let bekreftelse_message = context
            .avro_generator
            .create_avro_message("paw.arbeidssoker-bekreftelse-v1", bekreftelse)
            .await;

        // Steg 1: bekreftelsen ankommer før perioden finnes. Skal avvente (ikke persistere).
        let mut tx = context.start_tx().await;
        let first_attempt = context
            .bekreftelse_processor
            .process_payload(&mut tx, &bekreftelse_message)
            .await;
        assert!(matches!(
            first_attempt.expect("Forventet Ok(ProcessingOutcome::Pause) ved kaldstart"),
            ProcessorResult::Pause { .. }
        ));
        tx.rollback()
            .await
            .expect("Kunne ikke rulle tilbake første forsøk");

        let mut verify_tx = context.start_tx().await;
        let not_yet_persisted = bekreftelse::select_by_id(&mut verify_tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        verify_tx
            .commit()
            .await
            .expect("Kunne ikke commit transaksjon");
        assert!(
            not_yet_persisted.is_none(),
            "Bekreftelse skal ikke være persistert før periode er prosessert"
        );

        // Steg 2: perioden ankommer og prosesseres normalt.
        let periode = create_dummy_start_periode(identitetsnummer, periode_id, None);
        let periode_message = context
            .avro_generator
            .create_avro_message("paw.arbeidssokerperioder-v1", periode)
            .await;
        let mut periode_tx = context.start_tx().await;
        context
            .periode_processor
            .process_payload(&mut periode_tx, &periode_message)
            .await
            .expect("Periode-prosessering skal lykkes");
        periode_tx
            .commit()
            .await
            .expect("Kunne ikke commit periode-transaksjon");

        // Steg 3: bekreftelsen re-leveres (som etter pause+seek+resume i den ekte konsumentløkken)
        // og skal nå fullføre og kobles til riktig kartlegging.
        let mut retry_tx = context.start_tx().await;
        let retry_result = context
            .bekreftelse_processor
            .process_payload(&mut retry_tx, &bekreftelse_message)
            .await;
        assert!(retry_result.is_ok());
        let persisted_row = bekreftelse::select_by_id(&mut retry_tx, &bekreftelse_id)
            .await
            .expect("Kunne ikke hente bekreftelse");
        let kartlegging_rows = kartlegging::select_by_periode_id(&mut retry_tx, &periode_id)
            .await
            .expect("Kunne ikke hente kartlegging");
        retry_tx
            .commit()
            .await
            .expect("Kunne ikke commit retry-transaksjon");

        assert!(
            persisted_row.is_some(),
            "Bekreftelse skal nå være persistert"
        );
        let bekreftelse_row = persisted_row.expect("Ingen bekreftelse funnet");
        assert_eq!(bekreftelse_row.periode_id, periode_id);

        assert_eq!(kartlegging_rows.len(), 1);
        let kartlegging_row = kartlegging_rows.first().expect("Ingen kartlegging funnet");
        assert_eq!(kartlegging_row.periode_id, periode_id);
    }

    /// Regresjonstest for at `BekreftelseProcessor` overtar tidligere-kartlegging-fallbacken som
    /// tidligere kun `PeriodeProcessor` hadde. Dette er kritisk for rekalkuleringsplanen: siden
    /// kun bekreftelse-topicet rewindes (ikke periode-topicet), må `BekreftelseProcessor` alene
    /// kunne reprodusere ledigheten en tidligere periode ga videre.
    async fn test_utled_arbeidsledighet_fra_bekreftelse_tidligere_kartlegging(
        context: &TestContext,
    ) {
        let arbeidssoeker_id = 999_001;
        let aktor_id = "9999900001";
        let identitetsnummer = "99999900001";
        let tidligere_periode_id = Uuid::new_v4();
        let gjeldende_periode_id = Uuid::new_v4();

        let tidligere_periode_startet = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let tidligere_periode_avsluttet = tidligere_periode_startet + Duration::days(90);
        let tidligere_arbeidsledig_fra = tidligere_periode_startet + Duration::days(5);
        // Gjeldende periode starter kun 5 dager etter forrige periode ble avsluttet, altså godt
        // innenfor `periode_gap_grense_for_ledighet` (14 dager i test-konfigurasjonen).
        let gjeldende_periode_startet = tidligere_periode_avsluttet + Duration::days(5);

        let mut tx = context.start_tx().await;

        arbeidssoeker::insert(
            &mut tx,
            &ArbeidssoekerRow {
                id: arbeidssoeker_id,
                aktor_id: aktor_id.to_string(),
                identitetsnummer: identitetsnummer.to_string(),
                fornavn: None,
                mellomnavn: None,
                etternavn: None,
            },
        )
        .await
        .expect("Kunne ikke opprette arbeidssøker");

        kartlegging::insert(
            &mut tx,
            &KartleggingRow::new(
                tidligere_periode_id,
                arbeidssoeker_id,
                tidligere_periode_startet,
                Some(tidligere_periode_avsluttet),
                Some(tidligere_arbeidsledig_fra),
            ),
        )
        .await
        .expect("Kunne ikke opprette tidligere kartlegging");

        let gjeldende_kartlegging_row = KartleggingRow::new(
            gjeldende_periode_id,
            arbeidssoeker_id,
            gjeldende_periode_startet,
            None,
            None,
        );
        let bekreftelse_rows = vec![];

        // Bekreftelsen gjelder utelukkende før gjeldende periode ble startet, så
        // grensesnitts-regelen alene ville gitt `None` her.
        let hendelse = Bekreftelse {
            id: Uuid::new_v4(),
            periode_id: gjeldende_periode_id,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
            svar: Svar {
                sendt_inn_av: create_dummy_bekreftelse_metadata(identitetsnummer, None),
                gjelder_fra: gjeldende_periode_startet - Duration::days(10),
                gjelder_til: gjeldende_periode_startet - Duration::days(5),
                har_jobbet_i_denne_perioden: false,
                vil_fortsette_som_arbeidssoeker: true,
            },
        };

        let arbeidsledig_fra = utled_arbeidsledighet_for_bekreftelse(
            &mut tx,
            &hendelse,
            &gjeldende_kartlegging_row,
            &bekreftelse_rows,
            14,
        )
        .await
        .expect("Kunne ikke utlede ledighet");

        tx.rollback()
            .await
            .expect("Kunne ikke rulle tilbake transaksjon");

        assert_eq!(
            arbeidsledig_fra,
            Some(tidligere_arbeidsledig_fra),
            "Ledighet fra tidligere, nylig avsluttet periode skal overføres når bekreftelsen selv ikke gir noen ledighet"
        );
    }

    async fn init() -> TestContext {
        {
            let mut mockito_server = Server::new_async().await;

            let app_config = Arc::new(read_app_config().expect("Kunne ikke lese app_config.yaml"));

            let schema_registry_guard = create_schema_registry_mock(&mut mockito_server)
                .await
                .expect("Failed to create schema registry mock");
            let schema_registry_settings = schema_registry_guard.schema_registry_settings;

            let kafka_key_gen_mock_responses = default_kafka_key_gen_mock_responses();
            let kafka_key_gen_mock_guard =
                init_kafka_key_gen_api_mocks(&mut mockito_server, kafka_key_gen_mock_responses)
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
                    app_config.clone(),
                    schema_registry_settings.clone(),
                    key_gen_client,
                    pdl_client,
                ),
                bekreftelse_processor: BekreftelseProcessor::new(
                    app_config.clone(),
                    schema_registry_settings.clone(),
                ),
                arbeidssoeker_id_1: 12345,
                identitetsnummer_1: "01017012345",
                identitetsnummer_3: "02017012345",
                periode_id_1: Uuid::new_v4(),
                periode_id_3: Uuid::new_v4(),
                periode_id_4: Uuid::new_v4(),
                bekreftelse_id_1: Uuid::new_v4(),
                bekreftelse_id_2: Uuid::new_v4(),
                bekreftelse_id_3: Uuid::new_v4(),
                bekreftelse_id_4: Uuid::new_v4(),
            }
        }
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
        periode_id_4: Uuid,
        bekreftelse_id_1: Uuid,
        bekreftelse_id_2: Uuid,
        bekreftelse_id_3: Uuid,
        bekreftelse_id_4: Uuid,
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
