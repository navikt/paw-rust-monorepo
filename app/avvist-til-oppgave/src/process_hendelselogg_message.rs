use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::{hent_oppgave, insert_oppgave, insert_oppgave_hendelse_logg};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::Utc;
use interne_hendelser::Avvist;
use interne_hendelser::vo::{BrukerType, Opplysning};
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

pub async fn process_hendelselogg_message(
    kafka_message: &OwnedMessage,
    app_config: &ApplicationConfig,
    pg_pool: PgPool,
) -> anyhow::Result<()> {
    let hwm_version = *app_config.topic_hendelseslogg_version;
    let mut tx = pg_pool.begin().await?;

    if update_hwm(
        &mut tx,
        hwm_version,
        kafka_message.topic(),
        kafka_message.partition(),
        kafka_message.offset(),
    )
    .await?
    {
        opprett_oppgave_for_avvist_hendelse(kafka_message, app_config, &mut tx).await?;
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }

    Ok(())
}

async fn opprett_oppgave_for_avvist_hendelse(
    kafka_message: &OwnedMessage,
    app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let opprett_oppgaver_fra_tidspunkt = *app_config.opprett_oppgaver_fra_tidspunkt;
    let payload_bytes: Vec<u8> = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = serde_json::from_slice(&payload_bytes)?;
    let hendelse_type = json["hendelseType"].as_str().unwrap_or_default();
    let opplysninger: Vec<&str> = match json["opplysninger"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        None => Vec::new(),
    };

    if er_avvist_hendelse_under_18(hendelse_type, &opplysninger) {
        let avvist_hendelse: Avvist = serde_json::from_value(json.clone())?;
        if avvist_hendelse.metadata.utfoert_av.bruker_type == BrukerType::Veileder {
            tracing::info!(
                "Ignorerer hendelse av type: {} fordi den er innsendt av {}",
                hendelse_type,
                BrukerType::Veileder
            );
            return Ok(());
        }

        tracing::info!(
            "Håndterer melding på hendelselogg av type: {}",
            hendelse_type
        );

        let oppgave = hent_oppgave(avvist_hendelse.id, tx).await?;

        if skal_opprette_oppgave(&oppgave) {
            tracing::info!(
                "Sjekker dato: record={}, vannskille={}, resultat={}",
                avvist_hendelse.metadata.tidspunkt,
                opprett_oppgaver_fra_tidspunkt,
                avvist_hendelse.metadata.tidspunkt < opprett_oppgaver_fra_tidspunkt
            );
            if avvist_hendelse.metadata.tidspunkt < opprett_oppgaver_fra_tidspunkt {
                let oppgave_row = to_oppgave_row(
                    avvist_hendelse,
                    OppgaveType::AvvistUnder18,
                    OppgaveStatus::Ignorert,
                );
                let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;

                let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
                    oppgave_id,
                    status: HendelseLoggStatus::OppgaveIgnorert.to_string(),
                    melding: format!(
                        "Oppretter oppgave for avvist hendelse med status {} fordi hendelse er eldre enn {}",
                        OppgaveStatus::Ignorert.to_string(),
                        opprett_oppgaver_fra_tidspunkt
                    ),
                    tidspunkt: oppgave_row.tidspunkt.clone(),
                };

                insert_oppgave_hendelse_logg(&hendelse_logg_row, tx).await?;
            } else {
                let oppgave_row = to_oppgave_row(
                    avvist_hendelse,
                    OppgaveType::AvvistUnder18,
                    OppgaveStatus::Ubehandlet,
                );
                let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;

                let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
                    oppgave_id,
                    status: HendelseLoggStatus::OppgaveOpprettet.to_string(),
                    melding: "Oppretter oppgave for avvist hendelse".to_string(),
                    tidspunkt: oppgave_row.tidspunkt.clone(),
                };

                insert_oppgave_hendelse_logg(&hendelse_logg_row, tx).await?;
            }
        } else {
            let status_logg_row = InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave.unwrap().id,
                status: HendelseLoggStatus::AvvistHendelseMottatt.to_string(),
                melding: "Oppgave allerede opprettet for avvist hendelse for denne arbeidssoekeren"
                    .to_string(),
                tidspunkt: Utc::now(),
            };
            insert_oppgave_hendelse_logg(&status_logg_row, tx).await?;
        }
    } else {
        tracing::info!("Ignorerer irrelevant hendelse av type: {}", hendelse_type);
    }

    Ok(())
}

fn skal_opprette_oppgave(oppgave: &Option<Oppgave>) -> bool {
    match oppgave {
        None => true,
        Some(oppgave) => oppgave.status == OppgaveStatus::Ferdigbehandlet,
    }
}

fn er_avvist_hendelse_under_18(hendelse_type: &str, opplysninger: &[&str]) -> bool {
    hendelse_type == interne_hendelser::AVVIST_HENDELSE_TYPE
        && [Opplysning::ErUnder18Aar.to_string().as_str()]
            .iter()
            .all(|opp| opplysninger.contains(opp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_application_config;
    use anyhow::Result;
    use chrono::Utc;
    use health_and_monitoring::nais_otel_setup::setup_nais_otel;
    use interne_hendelser::Startet;
    use interne_hendelser::vo::{Bruker, Metadata};
    use paw_rdkafka_hwm::hwm_functions::insert_hwm;
    use paw_rust_base::convenience_functions::contains_all;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_process_hendelse() -> Result<()> {
        setup_nais_otel()?;

        let test_data = TestData::default();
        let start_hendelse = test_data.start_hendelse_string.as_bytes().to_vec();
        let avvist_hendelse_1 = test_data.avvist_hendelse_string.as_bytes().to_vec();
        let avvist_hendelse_2 = test_data.avvist_hendelse_string.as_bytes().to_vec();
        let avvist_hendelse_fra_veileder = test_data
            .avvist_hendelse_fra_veileder_string
            .as_bytes()
            .to_vec();

        let app_config = read_application_config()?;
        let hwm_version = *app_config.topic_hendelseslogg_version;
        let topic_hendelseslogg = app_config.topic_hendelseslogg.to_string();

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        //Blir vanligvis gjort av hwm_rebalance_listener
        insert_hwm(&mut tx, hwm_version, topic_hendelseslogg.as_str(), 0, 0).await?;
        tx.commit().await?;

        let irrelevant_message = OwnedMessage::new(
            Some(start_hendelse),
            None,
            topic_hendelseslogg.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        );

        let avvist_fra_veileder_message = OwnedMessage::new(
            Some(avvist_hendelse_fra_veileder),
            None,
            topic_hendelseslogg.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        let avvist_message_1 = OwnedMessage::new(
            Some(avvist_hendelse_1),
            None,
            topic_hendelseslogg.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            2,
            Some(OwnedHeaders::new()),
        );

        let avvist_message_2 = OwnedMessage::new(
            Some(avvist_hendelse_2),
            None,
            topic_hendelseslogg.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            3,
            Some(OwnedHeaders::new()),
        );

        // Skal ignorere irrelevante hendelser
        process_hendelselogg_message(&irrelevant_message, &app_config, pg_pool.clone()).await?;

        // Skal ignorerer avvist hendelse fra veileder
        process_hendelselogg_message(&avvist_fra_veileder_message, &app_config, pg_pool.clone())
            .await?;

        process_hendelselogg_message(&avvist_message_1, &app_config, pg_pool.clone()).await?;

        //Duplikat melding skal kun føre til en entry i status logg
        process_hendelselogg_message(&avvist_message_2, &app_config, pg_pool.clone()).await?;

        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = 12345;
        let oppgave = hent_oppgave(arbeidssoeker_id, &mut tx).await?.unwrap();

        assert_eq!(oppgave.type_, OppgaveType::AvvistUnder18);
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);
        assert_eq!(oppgave.hendelse_logg.len(), 2);
        assert!(
            contains_all(
                &oppgave.opplysninger,
                &[
                    Opplysning::ErUnder18Aar.to_string(),
                    Opplysning::BosattEtterFregLoven.to_string()
                ]
            ),
            "Mangler forventede opplysninger: {:?}",
            oppgave.opplysninger
        );
        assert_eq!(oppgave.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(oppgave.identitetsnummer, "12345678901");

        Ok(())
    }

    pub struct TestData {
        pub start_hendelse: Startet,
        pub avvist_hendelse: Avvist,
        pub avvist_hendelse_fra_veileder: Avvist,
        pub start_hendelse_string: &'static str,
        pub avvist_hendelse_string: &'static str,
        pub avvist_hendelse_fra_veileder_string: &'static str,
    }

    impl Default for TestData {
        fn default() -> Self {
            TestData {
                start_hendelse: Startet {
                    hendelse_id: Uuid::parse_str("701e247c-8c50-4ac1-8b29-d3f5e771da0c").unwrap(),
                    id: 0,
                    identitetsnummer: "".to_string(),
                    metadata: Metadata {
                        tidspunkt: Utc::now(),
                        utfoert_av: Bruker {
                            bruker_type: BrukerType::System,
                            id: "test.system".to_string(),
                            sikkerhetsnivaa: None,
                        },
                        kilde: "Testkilde".to_string(),
                        aarsak: "Mistet jobben".to_string(),
                        tidspunkt_fra_kilde: None,
                    },
                    opplysninger: vec![].into_iter().collect(),
                },
                avvist_hendelse: Avvist {
                    hendelse_id: Uuid::parse_str("cbbda03b-5fe5-48fd-a4ff-15605812f8cb").unwrap(),
                    id: 12345,
                    identitetsnummer: "01017012345".to_string(),
                    metadata: Metadata {
                        tidspunkt: Utc::now(),
                        utfoert_av: Bruker {
                            bruker_type: BrukerType::System,
                            id: "test.system".to_string(),
                            sikkerhetsnivaa: None,
                        },
                        kilde: "Testkilde".to_string(),
                        aarsak: "Er under 18 år".to_string(),
                        tidspunkt_fra_kilde: None,
                    },
                    opplysninger: vec![Opplysning::ErUnder18Aar, Opplysning::BosattEtterFregLoven]
                        .into_iter()
                        .collect(),
                    handling: None,
                },
                avvist_hendelse_fra_veileder: Avvist {
                    hendelse_id: Uuid::parse_str("723d5d09-83c7-4f83-97fd-35f7c9c5c798").unwrap(),
                    id: 12345,
                    identitetsnummer: "01017012345".to_string(),
                    metadata: Metadata {
                        tidspunkt: Utc::now(),
                        utfoert_av: Bruker {
                            bruker_type: BrukerType::Veileder,
                            id: "AA1234".to_string(),
                            sikkerhetsnivaa: None,
                        },
                        kilde: "Testkilde".to_string(),
                        aarsak: "Er under 18 år".to_string(),
                        tidspunkt_fra_kilde: None,
                    },
                    opplysninger: vec![Opplysning::ErUnder18Aar, Opplysning::BosattEtterFregLoven]
                        .into_iter()
                        .collect(),
                    handling: None,
                },
                start_hendelse_string: STARTET_HENDELSE,
                avvist_hendelse_string: AVVIST_HENDELSE_JSON,
                avvist_hendelse_fra_veileder_string: AVVIST_HENDELSE_FRA_VEILEDER_JSON,
            }
        }
    }

    //language=JSON
    const STARTET_HENDELSE: &str = r#"{
        "hendelseType": "intern.v1.startet",
        "opplysninger": ["TULL", "TØYS"]
    }"#;

    //language=JSON
    const AVVIST_HENDELSE_JSON: &str = r#"
        {
          "hendelseId": "723d5d09-83c7-4f83-97fd-35f7c9c5c798",
          "id": 12345,
          "identitetsnummer": "12345678901",
          "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
              "type": "SYSTEM",
              "id": "Testsystem"
            },
            "kilde": "Testkilde",
            "aarsak": "Er under 18 år"
          },
          "hendelseType": "intern.v1.avvist",
          "opplysninger": [
            "ER_UNDER_18_AAR",
            "BOSATT_ETTER_FREG_LOVEN"
          ]
        }
    "#;
    //language=JSON
    const AVVIST_HENDELSE_FRA_VEILEDER_JSON: &str = r#"
        {
          "hendelseId": "723d5d09-83c7-4f83-97fd-35f7c9c5c798",
          "id": 12345,
          "identitetsnummer": "12345678901",
          "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
              "type": "VEILEDER",
              "id": "Testsystem"
            },
            "kilde": "Testkilde",
            "aarsak": "Er under 18 år"
          },
          "hendelseType": "intern.v1.avvist",
          "opplysninger": [
            "ER_UNDER_18_AAR",
            "BOSATT_ETTER_FREG_LOVEN"
          ]
        }
    "#;
}
