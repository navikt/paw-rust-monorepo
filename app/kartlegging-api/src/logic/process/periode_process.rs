use crate::config::AppConfig;
use crate::logic::process::PayloadProcessor;
use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
use crate::model::dao::kartlegging::KartleggingRow;
use crate::model::dao::periode::PeriodeRow;
use crate::model::dao::{arbeidssoeker, bekreftelse, kartlegging, periode};
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::navn::Navn;
use crate::model::error::{DaoError, IdentityError, PayloadProcessorError};
use chrono::{DateTime, Utc};
use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_key_gen_client::model::IdentitetType;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use pdl_client::client::PDLClient;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;
use uuid::Uuid;

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
            hendelse.startet.tidspunkt,
            hendelse.avsluttet.as_ref().map(|m| m.tidspunkt),
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

    async fn utled_arbeidsledighet_fra_bekreftelser<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        periode_id: &'a Uuid,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        // Hent bekreftelser for periode-id
        let bekreftelse_rows = bekreftelse::select_by_periode_id(tx, periode_id).await?;

        let mut arbeidsledighet: Option<DateTime<Utc>> = None;
        // Loop igjennom bekreftelser og oppsummer ledighet
        for bekreftelse_row in bekreftelse_rows {
            if bekreftelse_row.har_jobbet {
                arbeidsledighet = None
            } else if !bekreftelse_row.har_jobbet && arbeidsledighet.is_none() {
                arbeidsledighet = Some(bekreftelse_row.gjelder_fra);
            }
        }

        Ok(arbeidsledighet)
    }

    async fn utled_arbeidsledighet_fra_eksisterende_kartlegging<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        periode_id: &'a Uuid,
        eksisterende_arbeidsledig_fra: &Option<DateTime<Utc>>,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        let arbeidsledighet = match eksisterende_arbeidsledig_fra.clone() {
            // Bruk ledighet fra eksisterende kartlegging
            Some(arbeidssledig_fra) => Some(arbeidssledig_fra),
            // Ingen ledighet fra eksisterende kartlegging, så prøv å utlede fra bekreftelser
            None => {
                self.utled_arbeidsledighet_fra_bekreftelser(tx, periode_id)
                    .await?
            }
        };

        Ok(arbeidsledighet)
    }

    async fn utled_arbeidsledighet_fra_tidligere_kartlegging<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        arbeidssoeker_id: &'a i64,
        periode_id: &'a Uuid,
        periode_startet: &'a DateTime<Utc>,
    ) -> anyhow::Result<Option<DateTime<Utc>>> {
        let periode_gap_grense = self.app_config.periode_gap_grense_for_ledighet;

        // Hent bekreftelser for periode-id
        let bekreftelser_arbeidsledig_fra = self
            .utled_arbeidsledighet_fra_bekreftelser(tx, periode_id)
            .await?;

        let arbeidsledighet = match bekreftelser_arbeidsledig_fra {
            // Om det finnes bekreftelser, bruk eventuell ledighet fra de
            Some(arbeidssledig_fra) => Some(arbeidssledig_fra),
            // Ingen bekreftelser for periode-id
            None => {
                // Søk etter tidligere kartlegging for arbeidssøker-id
                let tidligere_kartlegging_row =
                    kartlegging::select_latest_by_arbeidssoeker_id(tx, &arbeidssoeker_id).await?;

                match tidligere_kartlegging_row {
                    // Ingen tidligere kartlegging for arbeidssøker-id
                    None => None,
                    // Har en tidligere kartlegging for arbeidssøker-id, så hent eventuell ledighet fra den
                    Some(kartlegging_row) => {
                        match kartlegging_row.arbeidsledig_fra {
                            // Ingen ledighet satt for tidligere kartlegging
                            None => None,
                            Some(arbeidsledig_fra) => match kartlegging_row.arbeidssoeker_til {
                                // Tidligere periode er fortsatt aktiv. Dette er en feil!
                                None => None,
                                Some(arbeidssoeker_til) => {
                                    let periode_gap = periode_startet.clone() - arbeidssoeker_til;

                                    // Om det er mindre enn 14 dager siden tidligere periode ble avsluttet, bruk ledighet fra den
                                    if periode_gap.num_days() < periode_gap_grense {
                                        Some(arbeidsledig_fra)
                                    } else {
                                        None
                                    }
                                }
                            },
                        }
                    }
                }
            }
        };

        Ok(arbeidsledighet)
    }
}

impl PayloadProcessor for PeriodeProcessor {
    async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> anyhow::Result<(), ProcessorError> {
        match message.payload() {
            None => Err(PayloadProcessorError::no_payload_error(message).into()),
            Some(payload) => {
                let hendelse: Periode = self
                    .deserializer
                    .deserialize(payload)
                    .await
                    .map_err(|e| PayloadProcessorError::deserialization_error(message, &e))?;

                tracing::debug!("Mottok Periode-hendelse");

                // Lagre periode
                self.lagre_periode(tx, message, &hendelse).await?;

                // Hent identiteter fra Kafka Key Gen
                let arbeidssoeker = self
                    .hent_identiteter(message, &hendelse.identitetsnummer)
                    .await?;

                // Søk etter arbeidssøker(e)
                let arbeidssoeker_rows =
                    arbeidssoeker::select_by_arbeidssoeker_id(tx, &arbeidssoeker.id).await?;

                if arbeidssoeker_rows.len() > 1 {
                    // Mer enn én arbeidssøker funnet
                    Err(
                        DaoError::multiple_rows(message, "arbeidssøkere", arbeidssoeker_rows.len())
                            .into(),
                    )
                } else if arbeidssoeker_rows.len() == 1 {
                    // Arbeidssøker finnes fra før

                    let arbeidssoeker_row = arbeidssoeker_rows.first().ok_or_else(|| {
                        PayloadProcessorError::processing_error(
                            message,
                            "Fant ikke arbeidssøker i søkeresultat",
                        )
                    })?;

                    // Søk etter kartlegging(er)
                    let kartlegging_rows =
                        kartlegging::select_by_periode_id(tx, &hendelse.id).await?;

                    if kartlegging_rows.len() > 1 {
                        // Mer enn én kartlegging funnet
                        Err(DaoError::multiple_rows(
                            message,
                            "kartlegginger",
                            arbeidssoeker_rows.len(),
                        )
                        .into())
                    } else if kartlegging_rows.len() == 1 {
                        // Kartlegging finnes fra før

                        let kartlegging_row = kartlegging_rows
                            .first()
                            .ok_or_else(|| DaoError::no_rows(message, "kartlegginger"))?;

                        // Beregn ledighet fra eksisterende kartlegging
                        let arbeidsledig_fra = self
                            .utled_arbeidsledighet_fra_eksisterende_kartlegging(
                                tx,
                                &hendelse.id,
                                &kartlegging_row.arbeidsledig_fra,
                            )
                            .await?;

                        let arbeidssoeker_til = hendelse
                            .avsluttet
                            .map(|metadata| metadata.tidspunkt.clone());

                        // Lagre eksisterende kartlegging med arbeidssoeker_til og arbeidsledig_fra
                        kartlegging::update(
                            tx,
                            &hendelse.id,
                            &arbeidssoeker_til,
                            &arbeidsledig_fra,
                        )
                        .await?;

                        Ok(())
                    } else {
                        // Kartlegging finnes ikke fra før

                        // Beregn ledighet fra tidligere kartlegging, om den finnes
                        let arbeidsledig_fra = self
                            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                                tx,
                                &arbeidssoeker.id,
                                &hendelse.id,
                                &hendelse.startet.tidspunkt,
                            )
                            .await?;

                        let arbeidssoeker_til = hendelse
                            .avsluttet
                            .map(|metadata| metadata.tidspunkt.clone());

                        // Lagre ny kartlegging
                        let kartlegging_row = KartleggingRow::new(
                            hendelse.id.clone(),
                            arbeidssoeker_row.id,
                            hendelse.startet.tidspunkt.clone(),
                            arbeidssoeker_til,
                            arbeidsledig_fra,
                        );
                        kartlegging::insert(tx, &kartlegging_row).await?;

                        Ok(())
                    }
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

                    // Beregn ledighet fra tilhørende bekreftelser, om de finnes
                    let arbeidsledig_fra = self
                        .utled_arbeidsledighet_fra_bekreftelser(tx, &hendelse.id)
                        .await?;

                    let arbeidssoeker_til = hendelse
                        .avsluttet
                        .map(|metadata| metadata.tidspunkt.clone());

                    // Lagre ny kartlegging
                    let kartlegging_row = KartleggingRow::new(
                        hendelse.id.clone(),
                        arbeidssoeker.id,
                        hendelse.startet.tidspunkt.clone(),
                        arbeidssoeker_til,
                        arbeidsledig_fra,
                    );
                    kartlegging::insert(tx, &kartlegging_row).await?;

                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::read_app_config;
    use crate::logic::process::periode_process::PeriodeProcessor;
    use crate::logic::process::PayloadProcessor;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::dao::bekreftelse::BekreftelseRow;
    use crate::model::dao::kartlegging::KartleggingRow;
    use crate::model::dao::{arbeidssoeker, bekreftelse, kartlegging, periode};
    use chrono::{Duration, TimeZone, Utc};
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
        create_dummy_avsluttet_periode, create_dummy_startet_periode,
    };
    use token_client_stub::TokenClientStub;
    use tokio::sync::OnceCell;
    use tracing_test::traced_test;
    use uuid::Uuid;

    #[traced_test]
    #[tokio::test]
    async fn test_process_messages() {
        let context = init().await;
        test_process_periode_1_start(context).await;
        test_process_periode_1_avsluttet(context).await;
        test_process_periode_2_start(context).await;
    }

    async fn test_process_periode_1_start(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer_1 = context.identitetsnummer_1_1;
        let identitetsnummer_2 = context.identitetsnummer_1_2;
        let periode_id_1 = context.periode_id_1;

        let periode = create_dummy_startet_periode(identitetsnummer_1, periode_id_1);
        let message = context
            .avro_generator
            .create_avro_message(PAW_PERIODE_TOPIC, periode)
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
    }

    async fn test_process_periode_1_avsluttet(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer_2 = context.identitetsnummer_1_2;
        let periode_id_1 = context.periode_id_1;

        let periode = create_dummy_avsluttet_periode(identitetsnummer_2, periode_id_1);
        let message = context
            .avro_generator
            .create_avro_message(PAW_PERIODE_TOPIC, periode)
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
    }

    async fn test_process_periode_2_start(context: &TestContext) {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let identitetsnummer_2 = context.identitetsnummer_1_2;
        let periode_id_2 = context.periode_id_2;

        let periode = create_dummy_startet_periode(identitetsnummer_2, periode_id_2);
        let message = context
            .avro_generator
            .create_avro_message(PAW_PERIODE_TOPIC, periode)
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
    }

    //#[traced_test]
    #[tokio::test]
    async fn test_utled_arbeidsledighet() -> anyhow::Result<()> {
        let context = init().await;

        test_utled_arbeidsledighet_fra_bekreftelser(context).await?;
        test_utled_arbeidsledighet_fra_eksisterende_kartlegging(context).await?;
        test_utled_arbeidsledighet_fra_tidligere_kartlegging(context).await?;
        Ok(())
    }

    async fn test_utled_arbeidsledighet_fra_bekreftelser(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let periode_id = context.periode_id_3;
        let periode_startet = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

        let bekreftelse_row_1 = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet,
            gjelder_til: periode_startet + Duration::days(14),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let bekreftelse_row_2 = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet + Duration::days(14),
            gjelder_til: periode_startet + Duration::days(28),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let bekreftelse_row_3 = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet + Duration::days(28),
            gjelder_til: periode_startet + Duration::days(32),
            har_jobbet: true,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let mut tx = context.start_tx().await;

        let optional_ledighet_1 = context
            .processor
            .utled_arbeidsledighet_fra_bekreftelser(&mut tx, &periode_id)
            .await?;
        assert!(optional_ledighet_1.is_none());

        bekreftelse::insert(&mut tx, &bekreftelse_row_1).await?;

        let optional_ledighet_2 = context
            .processor
            .utled_arbeidsledighet_fra_bekreftelser(&mut tx, &periode_id)
            .await?;
        assert!(optional_ledighet_2.is_some());
        let ledighet_2 = optional_ledighet_2.expect("Ledighet ikke satt");
        assert_eq!(ledighet_2, periode_startet);

        bekreftelse::insert(&mut tx, &bekreftelse_row_2).await?;

        let optional_ledighet_3 = context
            .processor
            .utled_arbeidsledighet_fra_bekreftelser(&mut tx, &periode_id)
            .await?;
        assert!(optional_ledighet_3.is_some());
        let ledighet_3 = optional_ledighet_3.expect("Ledighet ikke satt");
        assert_eq!(ledighet_3, periode_startet);

        bekreftelse::insert(&mut tx, &bekreftelse_row_3).await?;

        let optional_ledighet_4 = context
            .processor
            .utled_arbeidsledighet_fra_bekreftelser(&mut tx, &periode_id)
            .await?;
        assert!(optional_ledighet_4.is_none());

        tx.commit().await.expect("Kunne ikke commit transaksjon");
        Ok(())
    }

    async fn test_utled_arbeidsledighet_fra_eksisterende_kartlegging(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let periode_id = context.periode_id_4;
        let periode_startet = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

        let bekreftelse_row = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet,
            gjelder_til: periode_startet + Duration::days(14),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let mut tx = context.start_tx().await;

        let optional_ledighet_1 = context
            .processor
            .utled_arbeidsledighet_fra_eksisterende_kartlegging(
                &mut tx,
                &periode_id,
                &Some(periode_startet + Duration::days(7)),
            )
            .await?;
        assert!(optional_ledighet_1.is_some());
        assert_eq!(
            optional_ledighet_1,
            Some(periode_startet + Duration::days(7))
        );

        let optional_ledighet_2 = context
            .processor
            .utled_arbeidsledighet_fra_eksisterende_kartlegging(&mut tx, &periode_id, &None)
            .await?;
        assert!(optional_ledighet_2.is_none());

        bekreftelse::insert(&mut tx, &bekreftelse_row).await?;

        let optional_ledighet_3 = context
            .processor
            .utled_arbeidsledighet_fra_eksisterende_kartlegging(&mut tx, &periode_id, &None)
            .await?;
        assert!(optional_ledighet_3.is_some());
        let ledighet_3 = optional_ledighet_3.expect("Ledighet ikke satt");
        assert_eq!(ledighet_3, bekreftelse_row.gjelder_fra);

        tx.commit().await.expect("Kunne ikke commit transaksjon");
        Ok(())
    }

    async fn test_utled_arbeidsledighet_fra_tidligere_kartlegging(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let arbeidssoeker_id = context.arbeidssoeker_id_5;
        let aktor_id = context.aktor_id_5;
        let identitetsnummer = context.identitetsnummer_5;
        let tidligere_periode_id = context.periode_id_5_1;
        let gjeldende_periode_id = context.periode_id_5_2;
        let tidligere_periode_startet = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let tidligere_periode_avsluttet = tidligere_periode_startet + Duration::days(90);

        let arbeidssoeker_row = ArbeidssoekerRow {
            id: arbeidssoeker_id,
            aktor_id: aktor_id.to_string(),
            identitetsnummer: identitetsnummer.to_string(),
            fornavn: None,
            mellomnavn: None,
            etternavn: None,
        };

        let tidligere_kartlegging_row = KartleggingRow {
            periode_id: tidligere_periode_id,
            arbeidssoeker_id,
            arbeidssoeker_fra: tidligere_periode_startet,
            arbeidssoeker_til: None,
            arbeidsledig_fra: None,
        };

        let arbeidsledig_fra_1 = tidligere_periode_startet + Duration::days(1);
        let arbeidsledig_fra_2 = tidligere_periode_startet + Duration::days(2);
        let arbeidsledig_fra_3 = tidligere_periode_startet + Duration::days(3);

        let bekreftelse_row = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id: gjeldende_periode_id,
            gjelder_fra: tidligere_periode_avsluttet + Duration::days(1),
            gjelder_til: tidligere_periode_avsluttet + Duration::days(15),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let mut tx = context.start_tx().await;

        arbeidssoeker::insert(&mut tx, &arbeidssoeker_row).await?;

        // Steg 1: Ingen bekreftelser og ingen tidligere kartlegginger
        let optional_ledighet_1 = context
            .processor
            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                &mut tx,
                &arbeidssoeker_id,
                &gjeldende_periode_id,
                &(tidligere_periode_avsluttet + Duration::days(10)),
            )
            .await?;
        assert!(optional_ledighet_1.is_none());

        // Steg 2: Har en tidligere kartlegging, men uten ledighet satt og periode er aktiv
        kartlegging::insert(&mut tx, &tidligere_kartlegging_row).await?;
        let optional_ledighet_2 = context
            .processor
            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                &mut tx,
                &arbeidssoeker_id,
                &gjeldende_periode_id,
                &(tidligere_periode_avsluttet + Duration::days(10)),
            )
            .await?;
        assert!(optional_ledighet_2.is_none());

        // Steg 3: Setter ledighet på tidligere kartlegging, men perioder er aktiv
        kartlegging::update(
            &mut tx,
            &tidligere_periode_id,
            &None,
            &Some(arbeidsledig_fra_1),
        )
        .await?;
        let optional_ledighet_3 = context
            .processor
            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                &mut tx,
                &arbeidssoeker_id,
                &gjeldende_periode_id,
                &(tidligere_periode_avsluttet + Duration::days(10)),
            )
            .await?;
        assert!(optional_ledighet_3.is_none());

        // Steg 4: Perioder er avluttet, men gap mellom perioder er mer enn 14 dager
        kartlegging::update(
            &mut tx,
            &tidligere_periode_id,
            &Some(tidligere_periode_avsluttet),
            &Some(arbeidsledig_fra_2),
        )
        .await?;
        let optional_ledighet_4 = context
            .processor
            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                &mut tx,
                &arbeidssoeker_id,
                &gjeldende_periode_id,
                &(tidligere_periode_avsluttet + Duration::days(15)),
            )
            .await?;
        assert!(optional_ledighet_4.is_none());

        // Steg 5: Perioder er avluttet, og gap mellom perioder er mindre enn 14 dager
        kartlegging::update(
            &mut tx,
            &tidligere_periode_id,
            &Some(tidligere_periode_avsluttet),
            &Some(arbeidsledig_fra_3),
        )
        .await?;
        let optional_ledighet_5 = context
            .processor
            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                &mut tx,
                &arbeidssoeker_id,
                &gjeldende_periode_id,
                &(tidligere_periode_avsluttet + Duration::days(13)),
            )
            .await?;
        assert_eq!(optional_ledighet_5, Some(arbeidsledig_fra_3));

        // Steg 6: Det finnes bekreftelse for gjeldende periode
        let gjeldende_periode_startet_6 = bekreftelse_row.gjelder_fra;
        bekreftelse::insert(&mut tx, &bekreftelse_row).await?;
        let optional_ledighet_6 = context
            .processor
            .utled_arbeidsledighet_fra_tidligere_kartlegging(
                &mut tx,
                &arbeidssoeker_id,
                &gjeldende_periode_id,
                &gjeldende_periode_startet_6,
            )
            .await?;
        assert_eq!(optional_ledighet_6, Some(gjeldende_periode_startet_6));

        tx.commit().await.expect("Kunne ikke commit transaksjon");
        Ok(())
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
                avro_generator: AvroGenerator::new(schema_registry_settings.clone()),
                processor: PeriodeProcessor::new(
                    app_config,
                    schema_registry_settings.clone(),
                    key_gen_client,
                    pdl_client,
                ),
                arbeidssoeker_id_1: 12345,
                arbeidssoeker_id_5: 56789,
                aktor_id_5: "501701234500",
                identitetsnummer_1_1: "41017012345",
                identitetsnummer_1_2: "01017012345",
                identitetsnummer_5: "05017012345",
                periode_id_1: Uuid::new_v4(),
                periode_id_2: Uuid::new_v4(),
                periode_id_3: Uuid::new_v4(),
                periode_id_4: Uuid::new_v4(),
                periode_id_5_1: Uuid::new_v4(),
                periode_id_5_2: Uuid::new_v4(),
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
        processor: PeriodeProcessor,
        arbeidssoeker_id_1: i64,
        arbeidssoeker_id_5: i64,
        aktor_id_5: &'static str,
        identitetsnummer_1_1: &'static str,
        identitetsnummer_1_2: &'static str,
        identitetsnummer_5: &'static str,
        periode_id_1: Uuid,
        periode_id_2: Uuid,
        periode_id_3: Uuid,
        periode_id_4: Uuid,
        periode_id_5_1: Uuid,
        periode_id_5_2: Uuid,
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
