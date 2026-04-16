use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use OppgaveStatus::{Ferdigbehandlet, Ubehandlet};
use OppgaveType::VurderOpphold;
use anyhow::Context;
use chrono::Utc;
use interne_hendelser::Startet;
use interne_hendelser::vo::{BrukerType, Opplysning};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use std::collections::HashSet;

pub async fn opprett_vurder_opphold_oppgave(
    json: Value,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let startet_hendelse: Startet =
        serde_json::from_value(json).context("Kunne ikke deserialisere startet hendelse")?;

    if startet_hendelse.metadata.utfoert_av.bruker_type != BrukerType::Sluttbruker {
        tracing::info!("Ignorerer startet-hendelse fordi den ikke er innsendt av sluttbruker");
        return Ok(());
    }

    if !vurder_opphold(&startet_hendelse.opplysninger) {
        tracing::info!(
            "Ignorerer startet hendelse — kriteriene for vurdering av opphold ikke oppfylt"
        );
        return Ok(());
    }

    let arbeidssoeker_id = startet_hendelse.id;
    let eksisterende_oppgave = hent_nyeste_oppgave(arbeidssoeker_id, VurderOpphold, tx).await?;
    if let Some(oppgave) = &eksisterende_oppgave
        && oppgave.status != Ferdigbehandlet
    {
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave.id,
                status: HendelseLoggStatus::OppgaveFinnesAllerede.to_string(),
                melding: "Arbeidssøkeren har allerede en aktiv vurder opphold oppgave".to_string(),
                tidspunkt: Utc::now(),
            },
            tx,
        )
        .await?;
        return Ok(());
    }

    let oppgave_row = to_oppgave_row(&startet_hendelse, VurderOpphold, Ubehandlet);
    let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;
    insert_oppgave_hendelse_logg(
        &InsertOppgaveHendelseLoggRow {
            oppgave_id,
            status: HendelseLoggStatus::OppgaveOpprettet.to_string(),
            melding: "Oppretter vurder opphold oppgave".to_string(),
            tidspunkt: oppgave_row.tidspunkt,
        },
        tx,
    )
    .await?;

    Ok(())
}

fn vurder_opphold(opplysninger: &HashSet<Opplysning>) -> bool {
    let ikke_bosatt = opplysninger.contains(&Opplysning::IkkeBosatt);
    let eu_eoes = opplysninger.contains(&Opplysning::ErEuEoesStatsborger);
    let norsk = opplysninger.contains(&Opplysning::ErNorskStatsborger);

    ikke_bosatt && eu_eoes && !norsk
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_application_config;
    use crate::db::oppgave_functions::{bytt_oppgave_status, hent_nyeste_oppgave};
    use crate::hendelselogg::process_hendelselogg_message;
    use anyhow::Result;
    use chrono::Utc;
    use paw_rust_base::convenience_functions::contains_all;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use std::collections::HashSet;

    #[test]
    fn test_er_vurder_opphold() {
        // Har IkkeBosatt + ErEuEoesStatsborger, ikke norsk
        assert!(vurder_opphold(&HashSet::from([
            Opplysning::IkkeBosatt,
            Opplysning::ErEuEoesStatsborger
        ])));

        // Mangler EU/EØS
        assert!(!vurder_opphold(&HashSet::from([Opplysning::IkkeBosatt])));

        // Er norsk statsborger — skal filtreres bort
        assert!(!vurder_opphold(&HashSet::from([
            Opplysning::IkkeBosatt,
            Opplysning::ErEuEoesStatsborger,
            Opplysning::ErNorskStatsborger
        ])));

        // Kun EU/EØS, mangler ikke-bosatt
        assert!(!vurder_opphold(&HashSet::from([
            Opplysning::ErEuEoesStatsborger
        ])));

        // SisteFlyttingVarUtAvNorge er ikke et gyldig kriterium
        assert!(!vurder_opphold(&HashSet::from([
            Opplysning::SisteFlyttingVarUtAvNorge,
            Opplysning::ErEuEoesStatsborger
        ])));

        // Tomt
        assert!(!vurder_opphold(&HashSet::new()));
    }

    #[tokio::test]
    async fn test_irrelevante_hendelser_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Startet hendelse uten relevante opplysninger
        let message = lag_kafka_melding(STARTET_HENDELSE_UTEN_RELEVANTE_OPPLYSNINGER);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "Skal ikke opprette oppgave for irrelevante hendelser"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_hendelse_fra_veileder_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message = lag_kafka_melding(STARTET_HENDELSE_FRA_VEILEDER);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "Skal ikke opprette oppgave for hendelse fra veileder"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_hendelse_fra_system_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message = lag_kafka_melding(STARTET_HENDELSE_FRA_SYSTEM);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "Skal ikke opprette oppgave for hendelse fra system"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Startet-flyt er midlertidig deaktivert i router.rs"]
    async fn test_startet_hendelse_oppretter_oppgave() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message = lag_kafka_melding(STARTET_HENDELSE_EU_EOES_IKKE_BOSATT);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.type_, VurderOpphold);
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.identitetsnummer, "12345678901");
        assert_eq!(oppgave.arbeidssoeker_id, 42);
        assert_eq!(oppgave.hendelse_logg.len(), 1);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveOpprettet
        );
        assert!(
            contains_all(
                &oppgave.opplysninger,
                &[
                    Opplysning::ErEuEoesStatsborger.to_string(),
                    Opplysning::IkkeBosatt.to_string()
                ]
            ),
            "Mangler forventede opplysninger: {:?}",
            oppgave.opplysninger
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_startet_hendelse_med_kun_utflyttet_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message = lag_kafka_melding(STARTET_HENDELSE_EU_EOES_UTFLYTTET);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(43, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "SisteFlyttingVarUtAvNorge er ikke lenger et gyldig kriterium — skal ikke opprette oppgave"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_norsk_statsborger_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message = lag_kafka_melding(STARTET_HENDELSE_NORSK_STATSBORGER);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(44, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "Norsk statsborger skal ikke opprette oppgave"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Startet-flyt er midlertidig deaktivert i router.rs"]
    async fn test_duplikat_hendelse_gir_logg_entry() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message_1 = lag_kafka_melding(STARTET_HENDELSE_EU_EOES_IKKE_BOSATT);
        let message_2 = lag_kafka_melding(STARTET_HENDELSE_EU_EOES_IKKE_BOSATT);

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_1, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_2, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.hendelse_logg.len(), 2);

        Ok(())
    }

    #[tokio::test]
    #[ignore = "Startet-flyt er midlertidig deaktivert i router.rs"]
    async fn test_ny_registrering_etter_ferdigbehandlet_startet_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let app_config = read_application_config()?;

        let message_1 = lag_kafka_melding(STARTET_HENDELSE_EU_EOES_IKKE_BOSATT);
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_1, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        bytt_oppgave_status(oppgave.id, Ubehandlet, Ferdigbehandlet, &mut tx).await?;
        tx.commit().await?;

        let message_2 = lag_kafka_melding(STARTET_HENDELSE_EU_EOES_IKKE_BOSATT);
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message_2, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(42, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveOpprettet
        );

        Ok(())
    }

    fn lag_kafka_melding(json: &str) -> OwnedMessage {
        OwnedMessage::new(
            Some(json.as_bytes().to_vec()),
            None,
            "test-topic".to_string(),
            Timestamp::CreateTime(Utc::now().timestamp_micros()),
            0,
            0,
            Some(OwnedHeaders::new()),
        )
    }

    //language=JSON
    const STARTET_HENDELSE_EU_EOES_IKKE_BOSATT: &str = r#"{
        "hendelseId": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
        "id": 42,
        "identitetsnummer": "12345678901",
        "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
                "type": "SLUTTBRUKER",
                "id": "12345678901"
            },
            "kilde": "Testkilde",
            "aarsak": "Test"
        },
        "hendelseType": "intern.v1.startet",
        "opplysninger": [
            "IKKE_BOSATT",
            "ER_EU_EOES_STATSBORGER"
        ]
    }"#;

    //language=JSON
    const STARTET_HENDELSE_EU_EOES_UTFLYTTET: &str = r#"{
        "hendelseId": "b2c3d4e5-f6a7-8901-bcde-f12345678901",
        "id": 43,
        "identitetsnummer": "12345678902",
        "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
                "type": "SLUTTBRUKER",
                "id": "12345678902"
            },
            "kilde": "Testkilde",
            "aarsak": "Test"
        },
        "hendelseType": "intern.v1.startet",
        "opplysninger": [
            "SISTE_FLYTTING_VAR_UT_AV_NORGE",
            "ER_EU_EOES_STATSBORGER"
        ]
    }"#;

    //language=JSON
    const STARTET_HENDELSE_NORSK_STATSBORGER: &str = r#"{
        "hendelseId": "c3d4e5f6-a7b8-9012-cdef-123456789012",
        "id": 44,
        "identitetsnummer": "12345678903",
        "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
                "type": "SLUTTBRUKER",
                "id": "12345678903"
            },
            "kilde": "Testkilde",
            "aarsak": "Test"
        },
        "hendelseType": "intern.v1.startet",
        "opplysninger": [
            "IKKE_BOSATT",
            "ER_EU_EOES_STATSBORGER",
            "ER_NORSK_STATSBORGER"
        ]
    }"#;

    //language=JSON
    const STARTET_HENDELSE_FRA_VEILEDER: &str = r#"{
        "hendelseId": "d4e5f6a7-b8c9-0123-defa-234567890123",
        "id": 42,
        "identitetsnummer": "12345678901",
        "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
                "type": "VEILEDER",
                "id": "Z991459"
            },
            "kilde": "Testkilde",
            "aarsak": "Test"
        },
        "hendelseType": "intern.v1.startet",
        "opplysninger": [
            "IKKE_BOSATT",
            "ER_EU_EOES_STATSBORGER"
        ]
    }"#;

    //language=JSON
    const STARTET_HENDELSE_FRA_SYSTEM: &str = r#"{
        "hendelseId": "d4e5f6a7-b8c9-0123-defa-234567890124",
        "id": 42,
        "identitetsnummer": "12345678901",
        "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
                "type": "SYSTEM",
                "id": "testsystem"
            },
            "kilde": "Testkilde",
            "aarsak": "Test"
        },
        "hendelseType": "intern.v1.startet",
        "opplysninger": [
            "IKKE_BOSATT",
            "ER_EU_EOES_STATSBORGER"
        ]
    }"#;

    //language=JSON
    const STARTET_HENDELSE_UTEN_RELEVANTE_OPPLYSNINGER: &str = r#"{
        "hendelseId": "e5f6a7b8-c9d0-1234-efab-345678901234",
        "id": 42,
        "identitetsnummer": "12345678901",
        "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
                "type": "SLUTTBRUKER",
                "id": "12345678901"
            },
            "kilde": "Testkilde",
            "aarsak": "Test"
        },
        "hendelseType": "intern.v1.startet",
        "opplysninger": [
            "BOSATT_ETTER_FREG_LOVEN",
            "ER_OVER_18_AAR"
        ]
    }"#;
}
