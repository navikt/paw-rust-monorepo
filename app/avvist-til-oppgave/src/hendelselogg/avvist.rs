use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use anyhow::Context;
use chrono::Utc;
use interne_hendelser::vo::{BrukerType, Opplysning};
use interne_hendelser::Avvist;
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use OppgaveStatus::{Ferdigbehandlet, Ignorert};

pub async fn opprett_oppgave_for_avvist_hendelse(
    json: Value,
    app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let opprett_oppgaver_fra_tidspunkt = *app_config.opprett_oppgaver_fra_tidspunkt;

    let avvist_hendelse: Avvist =
        serde_json::from_value(json).context("Kunne ikke deserialisere avvist hendelse")?;

    if avvist_hendelse.metadata.utfoert_av.bruker_type == BrukerType::Veileder {
        tracing::info!("Ignorerer hendelse fordi den er innsendt av veileder");
        return Ok(());
    }

    if avvist_hendelse.metadata.tidspunkt >= opprett_oppgaver_fra_tidspunkt {
        let arbeidssoeker_id = avvist_hendelse.id;
        let eksisterende_oppgave = hent_nyeste_oppgave(arbeidssoeker_id, tx).await?;
        if let Some(oppgave) = &eksisterende_oppgave
            && oppgave.status != Ferdigbehandlet
            && oppgave.status != Ignorert
        {
            insert_oppgave_hendelse_logg(
                &InsertOppgaveHendelseLoggRow {
                    oppgave_id: oppgave.id,
                    status: HendelseLoggStatus::OppgaveFinnesAllerede.to_string(),
                    melding: "Arbeidssøkeren har allerede en aktiv oppgave for avvist registrering"
                        .to_string(),
                    tidspunkt: Utc::now(),
                },
                tx,
            )
            .await?;
            return Ok(());
        }

        let oppgave_row = to_oppgave_row(
            &avvist_hendelse,
            OppgaveType::AvvistUnder18,
            OppgaveStatus::Ubehandlet,
        );
        let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: HendelseLoggStatus::OppgaveOpprettet.to_string(),
                melding: "Oppretter oppgave for avvist hendelse".to_string(),
                tidspunkt: oppgave_row.tidspunkt,
            },
            tx,
        )
        .await?;
    } else {
        let oppgave_row = to_oppgave_row(&avvist_hendelse, OppgaveType::AvvistUnder18, Ignorert);
        let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: HendelseLoggStatus::OppgaveIgnorert.to_string(),
                melding: format!(
                    "Oppretter oppgave for avvist hendelse med status {} fordi hendelse er eldre enn {}",
                    Ignorert,
                    opprett_oppgaver_fra_tidspunkt
                ),
                tidspunkt: oppgave_row.tidspunkt,
            },
            tx,
        )
        .await?;
    }

    Ok(())
}

pub fn er_avvist_hendelse_under_18(hendelse_type: &str, opplysninger: &[&str]) -> bool {
    hendelse_type == interne_hendelser::AVVIST_HENDELSE_TYPE
        && opplysninger.contains(&Opplysning::ErUnder18Aar.to_string().as_str())
}

#[cfg(test)]
mod tests {
    use crate::config::read_application_config;
    use crate::db::oppgave_functions::{bytt_oppgave_status, hent_nyeste_oppgave};
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use crate::domain::oppgave_status::OppgaveStatus;
    use crate::domain::oppgave_type::OppgaveType;
    use crate::hendelselogg::process_hendelselogg_message;
    use anyhow::Result;
    use chrono::Utc;
    use interne_hendelser::vo::Opplysning;
    use paw_rust_base::convenience_functions::contains_all;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use OppgaveStatus::Ferdigbehandlet;

    #[tokio::test]
    async fn test_process_hendelse() -> Result<()> {
        let test_data = TestData::default();
        let start_hendelse = test_data.start_hendelse_string.as_bytes().to_vec();
        let avvist_hendelse_1 = test_data.avvist_hendelse_string.as_bytes().to_vec();
        let avvist_hendelse_2 = test_data.avvist_hendelse_string.as_bytes().to_vec();
        let avvist_hendelse_fra_veileder = test_data
            .avvist_hendelse_fra_veileder_string
            .as_bytes()
            .to_vec();

        let app_config = read_application_config()?;
        let topic = "test-topic";

        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let irrelevant_message = OwnedMessage::new(
            Some(start_hendelse),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        );

        let avvist_fra_veileder_message = OwnedMessage::new(
            Some(avvist_hendelse_fra_veileder),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        let avvist_message_1 = OwnedMessage::new(
            Some(avvist_hendelse_1),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            2,
            Some(OwnedHeaders::new()),
        );

        let avvist_message_2 = OwnedMessage::new(
            Some(avvist_hendelse_2),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            3,
            Some(OwnedHeaders::new()),
        );

        // Skal ignorere irrelevante hendelser
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&irrelevant_message, &app_config, &mut tx).await?;
        tx.commit().await?;

        // Skal ignorere avvist hendelse fra veileder
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&avvist_fra_veileder_message, &app_config, &mut tx)
            .await?;
        tx.commit().await?;

        // Skal opprette oppgave for avvist hendelse
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&avvist_message_1, &app_config, &mut tx).await?;
        tx.commit().await?;

        // Duplikat melding skal kun føre til en entry i status logg
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&avvist_message_2, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = 12345;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id, &mut tx)
            .await?
            .unwrap();

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

    #[tokio::test]
    async fn test_gammel_hendelse_gir_ignorert_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut app_config = read_application_config()?;
        app_config.opprett_oppgaver_fra_tidspunkt =
            chrono::DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc)
                .into();

        let avvist_hendelse = AVVIST_HENDELSE_JSON.as_bytes().to_vec();
        let message = OwnedMessage::new(
            Some(avvist_hendelse),
            None,
            "test-topic".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        );

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(12345, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, OppgaveStatus::Ignorert);
        assert_eq!(oppgave.hendelse_logg.len(), 1);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveIgnorert
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_ny_registrering_etter_ignorert_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut app_config = read_application_config()?;
        app_config.opprett_oppgaver_fra_tidspunkt =
            chrono::DateTime::parse_from_rfc3339("2030-01-01T00:00:00Z")?
                .with_timezone(&Utc)
                .into();

        let avvist_hendelse = AVVIST_HENDELSE_JSON.as_bytes().to_vec();
        let message = OwnedMessage::new(
            Some(avvist_hendelse),
            None,
            "test-topic".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        );

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(12345, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, OppgaveStatus::Ignorert);
        tx.commit().await?;

        let app_config = read_application_config()?;
        let avvist_hendelse_2 = AVVIST_HENDELSE_JSON.as_bytes().to_vec();
        let message_2 = OwnedMessage::new(
            Some(avvist_hendelse_2),
            None,
            "test-topic".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_2, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(12345, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveOpprettet
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_ny_registrering_etter_ferdigbehandlet_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let app_config = read_application_config()?;
        let topic = "test-topic";

        let avvist_hendelse_1 = AVVIST_HENDELSE_JSON.as_bytes().to_vec();
        let avvist_hendelse_2 = AVVIST_HENDELSE_JSON.as_bytes().to_vec();

        let message_1 = OwnedMessage::new(
            Some(avvist_hendelse_1),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        );

        // Opprett første oppgave
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_1, &app_config, &mut tx).await?;
        tx.commit().await?;

        // Sett oppgaven til Ferdigbehandlet
        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(12345, &mut tx).await?.unwrap();
        bytt_oppgave_status(
            oppgave.id,
            OppgaveStatus::Ubehandlet,
            Ferdigbehandlet,
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        // Send ny avvist hendelse for samme arbeidssøker — skal opprette ny oppgave
        let message_2 = OwnedMessage::new(
            Some(avvist_hendelse_2),
            None,
            topic.to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            1,
            Some(OwnedHeaders::new()),
        );

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_2, &app_config, &mut tx).await?;
        tx.commit().await?;

        // Verifiser at ny oppgave ble opprettet (hent_nyeste_oppgave henter den nyeste)
        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(12345, &mut tx).await?.unwrap();
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveOpprettet
        );

        Ok(())
    }

    pub struct TestData {
        pub start_hendelse_string: &'static str,
        pub avvist_hendelse_string: &'static str,
        pub avvist_hendelse_fra_veileder_string: &'static str,
    }

    impl Default for TestData {
        fn default() -> Self {
            TestData {
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
