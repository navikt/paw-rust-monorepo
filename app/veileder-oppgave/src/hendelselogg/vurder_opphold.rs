use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::Utc;
use interne_hendelser::Startet;
use paw_rust_base::env::{runtime_env, RuntimeEnv};
use sqlx::{Postgres, Transaction};
use OppgaveStatus::{Ferdigbehandlet, Ubehandlet};
use OppgaveType::VurderOpphold;
use crate::domain::kriterier::vurder_opphold;

pub async fn opprett_vurder_opphold_oppgave(
    startet_hendelse: &Startet,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    if !vurder_opphold::KRITERIER.oppfylt_av(startet_hendelse) {
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

    let oppgave_row = to_oppgave_row(startet_hendelse, VurderOpphold, Ubehandlet);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_application_config;
    use crate::db::oppgave_functions::{bytt_oppgave_status, hent_nyeste_oppgave};
    use crate::hendelselogg::process_hendelselogg_message;

    use anyhow::Result;
    use interne_hendelser::vo::{BrukerType, Opplysning};
    use interne_hendelser::Startet;
    use paw_rust_base::convenience_functions::contains_all;
    use paw_test::hendelse_builder::{AsJson, StartetBuilder};
    use paw_test::setup_test_db::setup_test_db;
    use std::collections::HashSet;

    const ARBEIDSSOEKER_ID: i64 = 42;
    const IDENTITETSNUMMER: &str = "12345678901";

    #[tokio::test]
    async fn test_irrelevante_hendelser_ignoreres() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let hendelse: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            utfoert_av_id: IDENTITETSNUMMER.to_string(),
            opplysninger: HashSet::from([
                Opplysning::BosattEtterFregLoven,
                Opplysning::ErOver18Aar,
            ]),
            ..Default::default()
        }
        .build();
        let message = hendelse.as_json();

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARBEIDSSOEKER_ID, VurderOpphold, &mut tx).await?;
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

        let fra_veileder: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            bruker_type: BrukerType::Veileder,
            utfoert_av_id: "Z991459".to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        let fra_system: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            bruker_type: BrukerType::System,
            utfoert_av_id: "Testsystem".to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();

        let fra_veileder_message = fra_veileder.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(fra_veileder_message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let fra_system_message = fra_system.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(fra_system_message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARBEIDSSOEKER_ID, VurderOpphold, &mut tx).await?;
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

        let hendelse: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            utfoert_av_id: IDENTITETSNUMMER.to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        let message = hendelse.as_json();

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARBEIDSSOEKER_ID, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.type_, VurderOpphold);
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(oppgave.identitetsnummer, IDENTITETSNUMMER);
        assert_eq!(oppgave.arbeidssoeker_id, ARBEIDSSOEKER_ID);
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

        let hendelse: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            utfoert_av_id: IDENTITETSNUMMER.to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        let message = hendelse.as_json();

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARBEIDSSOEKER_ID, VurderOpphold, &mut tx)
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

        let hendelse_1: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            utfoert_av_id: IDENTITETSNUMMER.to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        let message = hendelse_1.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARBEIDSSOEKER_ID, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        bytt_oppgave_status(oppgave.id, Ubehandlet, Ferdigbehandlet, &mut tx).await?;
        tx.commit().await?;

        let hendelse_2: Startet = StartetBuilder {
            arbeidssoeker_id: ARBEIDSSOEKER_ID,
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            utfoert_av_id: IDENTITETSNUMMER.to_string(),
            opplysninger: HashSet::from([Opplysning::IkkeBosatt, Opplysning::ErEuEoesStatsborger]),
            ..Default::default()
        }
        .build();
        let message = hendelse_2.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARBEIDSSOEKER_ID, VurderOpphold, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, Ubehandlet);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveOpprettet
        );

        Ok(())
    }
}
