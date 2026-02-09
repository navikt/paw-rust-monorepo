use crate::avvist_hendelse::AvvistHendelse;
use crate::db::oppgave_functions::{hent_oppgave, insert_oppgave_med};
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use health_and_monitoring::simple_app_state::AppState;
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
        let avvist_hendelse: AvvistHendelse = serde_json::from_value(json)?;
        let oppgave = hent_oppgave(avvist_hendelse.id, tx).await?;
        if oppgave.is_none() { //TODO, riktig kriterie? Flere oppgaver? Spesifikk status på oppgave?
            let oppgave_row = to_oppgave_row(avvist_hendelse, OppgaveType::AvvistUnder18);
            insert_oppgave_med(OppgaveStatus::Ubehandlet, &oppgave_row, tx).await?;
        } else { 
            oppdater_status_logg(oppgave.unwrap().id, avvist_hendelse, tx).await?;
        }
        log::info!("Prosesserer avvist hendelse for arbeidssøker");
        tracing::info!("Prosesserer avvist hendelse for arbeidssølker");
    }
    Ok(())
}

const AVVIST_HENDELSE_TYPE: &str = "intern.v1.avvist";
const OPPLYSNING_UNDER_18: &str = "ER_UNDER_18_AAR";
const BOSATT_ETTER_FREG_LOVEN: &str = "BOSATT_ETTER_FREG_LOVEN";

fn er_avvist_hendelse_under_18(hendelse_type: &str, opplysninger: &[&str]) -> bool {
    hendelse_type == AVVIST_HENDELSE_TYPE
        && [BOSATT_ETTER_FREG_LOVEN, OPPLYSNING_UNDER_18]
            .iter()
            .all(|opp| opplysninger.contains(opp))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_row::OppgaveRow;
    use chrono::Utc;
    use paw_rdkafka_hwm::hwm_functions::insert_hwm;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};

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

        let avvist_message = OwnedMessage::new(
            Some(AVVIST_HENDELSE_JSON.as_bytes().to_vec()),
            None,
            "hendelselogg".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        let tx = pg_pool.begin().await?;
        process_hendelse(&irrelevant_message, tx).await?;

        let tx = pg_pool.begin().await?;
        process_hendelse(&avvist_message, tx).await?;

        let oppgave_row: OppgaveRow = sqlx::query_as(
            r#"
                    SELECT
                        id,
                        type as type_,
                        melding_id,
                        opplysninger,
                        arbeidssoeker_id,
                        identitetsnummer,
                        tidspunkt AT TIME ZONE 'UTC' as tidspunkt
                    FROM oppgaver
                    "#,
        )
        .fetch_one(&pg_pool)
        .await?;
        assert_eq!(oppgave_row.type_, OppgaveType::AvvistUnder18.to_string());
        assert_eq!(
            oppgave_row.opplysninger,
            vec!["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN"]
        );
        assert_eq!(oppgave_row.arbeidssoeker_id, 12345);
        assert_eq!(oppgave_row.identitetsnummer, "12345678901");

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
}
