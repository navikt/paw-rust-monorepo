use crate::db::oppgave_functions::{hent_oppgave, insert_oppgave, insert_oppgave_hendelse_logg};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::Utc;
use health_and_monitoring::simple_app_state::AppState;
use interne_hendelser;
use interne_hendelser::Avvist;
use interne_hendelser::vo::BrukerType;
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use rdkafka::message::Message;
use rdkafka::message::OwnedMessage;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::error::Error;
use std::sync::Arc;

pub async fn start_processing_loop(
    hendelselogg_consumer: StreamConsumer<HwmRebalanceHandler>,
    pg_pool: PgPool,
    _app_state: Arc<AppState>,
) -> Result<(), Box<dyn Error>> {
    log::info!("Starting processing loop");
    loop {
        let kafka_message = hendelselogg_consumer.recv().await?.detach();
        let tx = pg_pool.begin().await?;
        process_hendelse(&kafka_message, tx).await?;
    }
}

const HWM_VERSION: i16 = 1;

pub async fn process_hendelse(
    kafka_message: &OwnedMessage,
    mut tx: Transaction<'_, Postgres>,
) -> Result<(), Box<dyn Error>> {
    if update_hwm(
        &mut tx,
        HWM_VERSION,
        kafka_message.topic(),
        kafka_message.partition(),
        kafka_message.offset(),
    )
    .await?
    {
        lag_oppgave_for_avvist_hendelse(kafka_message, &mut tx).await?;
        tx.commit().await?;
    } else {
        tx.rollback().await?;
    }
    Ok(())
}

async fn lag_oppgave_for_avvist_hendelse(
    kafka_message: &OwnedMessage,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), Box<dyn Error>> {
    let payload_bytes: Vec<u8> = kafka_message.payload().unwrap_or(&[]).to_vec();
    let json: Value = serde_json::from_slice(&payload_bytes)?;
    let hendelse_type = json["hendelseType"].as_str().unwrap_or_default();
    let opplysninger: Vec<&str> = match json["opplysninger"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect(),
        None => Vec::new(),
    };

    if er_avvist_hendelse_under_18(hendelse_type, &opplysninger) {
        let avvist_hendelse: Avvist = serde_json::from_value(json)?;
        if avvist_hendelse.metadata.utfoert_av.bruker_type == BrukerType::Veileder {
            return Ok(());
        }
        let oppgave = hent_oppgave(avvist_hendelse.id, tx).await?;

        if skal_opprette_oppgave(&oppgave) {
            let oppgave_row = to_oppgave_row(
                avvist_hendelse,
                OppgaveType::AvvistUnder18,
                OppgaveStatus::Ubehandlet,
            );
            insert_oppgave(&oppgave_row, tx).await?;
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
    }
    Ok(())
}

fn skal_opprette_oppgave(oppgave: &Option<Oppgave>) -> bool {
    match oppgave {
        None => true,
        Some(oppgave) => oppgave.status == OppgaveStatus::Ferdigbehandlet,
    }
}

const OPPLYSNING_UNDER_18: &str = "ER_UNDER_18_AAR";

fn er_avvist_hendelse_under_18(hendelse_type: &str, opplysninger: &[&str]) -> bool {
    hendelse_type == interne_hendelser::AVVIST_HENDELSE_TYPE
        && [OPPLYSNING_UNDER_18]
            .iter()
            .all(|opp| opplysninger.contains(opp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paw_rdkafka_hwm::hwm_functions::insert_hwm;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use paw_rust_base::convenience_functions::contains_all;

    #[tokio::test]
    async fn test_process_hendelse() -> Result<(), Box<dyn Error>> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        //Blir vanligvis gjort av hwm_rebalance_listener
        insert_hwm(&mut tx, HWM_VERSION, "hendelselogg", 0, 0).await?;
        tx.commit().await?;

        let irrelevant_message = OwnedMessage::new(
            Some(STARTET_HENDELSE.as_bytes().to_vec()),
            None,
            "hendelselogg".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        );

        let avvist_fra_veileder_message = OwnedMessage::new(
            Some(AVVIST_HENDELSE_FRA_VEILEDER_JSON.as_bytes().to_vec()),
            None,
            "hendelselogg".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        let avvist_message = OwnedMessage::new(
            Some(AVVIST_HENDELSE_JSON.as_bytes().to_vec()),
            None,
            "hendelselogg".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            2,
            Some(OwnedHeaders::new()),
        );

        let andre_avvist_message = OwnedMessage::new(
            Some(AVVIST_HENDELSE_JSON.as_bytes().to_vec()),
            None,
            "hendelselogg".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            3,
            Some(OwnedHeaders::new()),
        );

        let tx = pg_pool.begin().await?;
        process_hendelse(&irrelevant_message, tx).await?;

        // Skal ikke prosessere avvist hendelse fra veileder
        let tx = pg_pool.begin().await?;
        process_hendelse(&avvist_fra_veileder_message, tx).await?;

        let tx = pg_pool.begin().await?;
        process_hendelse(&avvist_message, tx).await?;

        //Duplikat melding skal kun føre til en entry i status logg
        let tx = pg_pool.begin().await?;
        process_hendelse(&andre_avvist_message, tx).await?;

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
                    "ErUnder18Aar".to_string(),
                    "BosattEtterFregLoven".to_string()
                ]
            ),
            "Mangler forventede opplysninger: {:?}",
            oppgave.opplysninger
        );
        assert_eq!(oppgave.arbeidssoeker_id, arbeidssoeker_id);
        assert_eq!(oppgave.identitetsnummer, "12345678901");

        Ok(())
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
