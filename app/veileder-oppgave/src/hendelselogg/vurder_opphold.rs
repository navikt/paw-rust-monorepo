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
use paw_rust_base::env::{RuntimeEnv, runtime_env};
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

    // TODO: Midlertidig tripwire — fjernes etter helga :tm:
    if runtime_env() == RuntimeEnv::ProdGcp {
        panic!(
            "Uventet startet hendelse som innfrir vurder_opphold oppgave kriteriene. HendelseId={}. \
             Dette skal egentlig ikke skje. Consumer stoppet med vilje.",
            startet_hendelse.hendelse_id
        );
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
    let utflyttet = opplysninger.contains(&Opplysning::IkkeBosatt);
    let eu_eoes_statsborger = opplysninger.contains(&Opplysning::ErEuEoesStatsborger);
    let ikke_norsk_statsborger = !opplysninger.contains(&Opplysning::ErNorskStatsborger);

    utflyttet && eu_eoes_statsborger && ikke_norsk_statsborger
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_application_config;
    use crate::db::oppgave_functions::{bytt_oppgave_status, hent_nyeste_oppgave};
    use crate::hendelselogg::process_hendelselogg_message;
    use anyhow::Result;
    use chrono::Utc;
    use interne_hendelser::Startet;
    use paw_rust_base::convenience_functions::contains_all;
    use paw_test::hendelse_builder::{AsJson, StartetBuilder};
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
    use std::collections::HashSet;

    const ARB_ID: i64 = 42;
    const IDENT: &str = "12345678901";

    fn startet_vurder_opphold_builder() -> StartetBuilder {
        StartetBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            utfoert_av_id: IDENT.to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
    }

    fn startet_vurder_opphold() -> Startet {
        startet_vurder_opphold_builder().build()
    }

    #[test]
    fn test_er_vurder_opphold() {
        assert!(vurder_opphold(&HashSet::from([
            Opplysning::IkkeBosatt,
            Opplysning::ErEuEoesStatsborger
        ])));
        assert!(!vurder_opphold(&HashSet::from([Opplysning::IkkeBosatt])));
        assert!(!vurder_opphold(&HashSet::from([
            Opplysning::IkkeBosatt,
            Opplysning::ErEuEoesStatsborger,
            Opplysning::ErNorskStatsborger
        ])));
        assert!(!vurder_opphold(&HashSet::from([
            Opplysning::ErEuEoesStatsborger
        ])));
        assert!(!vurder_opphold(&HashSet::from([
            Opplysning::SisteFlyttingVarUtAvNorge,
            Opplysning::ErEuEoesStatsborger
        ])));
        assert!(!vurder_opphold(&HashSet::new()));
    }

    #[tokio::test]
    async fn test_irrelevante_hendelser_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let hendelse: Startet = StartetBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            utfoert_av_id: IDENT.to_string(),
            opplysninger: HashSet::from([
                Opplysning::BosattEtterFregLoven,
                Opplysning::ErOver18Aar,
            ]),
            ..Default::default()
        }
        .build();
        let message = lag_kafka_melding(&hendelse.as_json());

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "Skal ikke opprette oppgave for irrelevante hendelser"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_hendelse_ikke_fra_sluttbruker_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let hendelser: [Startet; 2] = [
            StartetBuilder {
                bruker_type: BrukerType::Veileder,
                utfoert_av_id: "Z991459".to_string(),
                ..startet_vurder_opphold_builder()
            }
            .build(),
            StartetBuilder {
                bruker_type: BrukerType::System,
                utfoert_av_id: "Testsystem".to_string(),
                ..startet_vurder_opphold_builder()
            }
            .build(),
        ];

        for hendelse in hendelser {
            let message = lag_kafka_melding(&hendelse.as_json());
            let mut tx = pg_pool.begin().await?;
            process_hendelselogg_message(&message, &app_config, &mut tx).await?;
            tx.commit().await?;
        }

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, VurderOpphold, &mut tx).await?;
        assert!(
            oppgave.is_none(),
            "Skal ikke opprette oppgave for hendelser som ikke er fra sluttbruker"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_startet_hendelse_oppretter_oppgave() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let message = lag_kafka_melding(&startet_vurder_opphold().as_json());

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.type_, VurderOpphold);
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.identitetsnummer, IDENT);
        assert_eq!(oppgave.arbeidssoeker_id, ARB_ID);
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
    async fn test_duplikat_hendelse_gir_logg_entry() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        for _ in 0..2 {
            let message = lag_kafka_melding(&startet_vurder_opphold().as_json());
            let mut tx = pg_pool.begin().await?;
            process_hendelselogg_message(&message, &app_config, &mut tx).await?;
            tx.commit().await?;
        }

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.hendelse_logg.len(), 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_ny_registrering_etter_ferdigbehandlet_startet_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let app_config = read_application_config()?;

        let message = lag_kafka_melding(&startet_vurder_opphold().as_json());
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        bytt_oppgave_status(oppgave.id, Ubehandlet, Ferdigbehandlet, &mut tx).await?;
        tx.commit().await?;

        let message = lag_kafka_melding(&startet_vurder_opphold().as_json());
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(&message, &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, VurderOpphold, &mut tx)
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
}
