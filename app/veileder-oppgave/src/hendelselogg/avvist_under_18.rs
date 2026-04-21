use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::kriterier::avvist_under_18;
use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use chrono::Utc;
use interne_hendelser::Avvist;
use sqlx::{Postgres, Transaction};
use OppgaveStatus::{Ferdigbehandlet, Ignorert};

pub async fn opprett_avvist_under_18_oppgave(
    avvist_hendelse: &Avvist,
    app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let opprett_avvist_under_18_oppgaver_fra_tidspunkt =
        *app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt;

    if !avvist_under_18::KRITERIER.oppfylt_av(avvist_hendelse) {
        return Ok(());
    }

    if avvist_hendelse.metadata.tidspunkt >= opprett_avvist_under_18_oppgaver_fra_tidspunkt {
        let arbeidssoeker_id = avvist_hendelse.id;
        let eksisterende_oppgave = hent_nyeste_oppgave(arbeidssoeker_id, OppgaveType::AvvistUnder18, tx).await?;
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
            avvist_hendelse,
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
        let oppgave_row = to_oppgave_row(avvist_hendelse, OppgaveType::AvvistUnder18, Ignorert);
        let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: HendelseLoggStatus::OppgaveIgnorert.to_string(),
                melding: format!(
                    "Oppretter oppgave for avvist hendelse med status {} fordi hendelse er eldre enn {}",
                    Ignorert,
                    opprett_avvist_under_18_oppgaver_fra_tidspunkt
                ),
                tidspunkt: oppgave_row.tidspunkt,
            },
            tx,
        )
        .await?;
    }

    Ok(())
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
    use interne_hendelser::vo::Opplysning::ErUnder18Aar;
    use interne_hendelser::vo::{BrukerType, Opplysning};
    use interne_hendelser::{Avvist, Startet};
    use paw_rust_base::convenience_functions::contains_all;
    use paw_test::hendelse_builder::{rfc3339, AsJson, AvvistBuilder, StartetBuilder};
    use paw_test::setup_test_db::setup_test_db;
    use std::collections::HashSet;
    use OppgaveStatus::Ferdigbehandlet;
    use Opplysning::BosattEtterFregLoven;

    const ARB_ID: i64 = 12345;
    const IDENT: &str = "12345678901";

    #[tokio::test]
    async fn test_process_hendelse() -> Result<()> {
        let app_config = read_application_config()?;
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let irrelevant_hendelse: Startet = StartetBuilder {
            arbeidssoeker_id: 99,
            identitetsnummer: "99999999999".to_string(),
            utfoert_av_id: "99999999999".to_string(),
            opplysninger: HashSet::from([BosattEtterFregLoven, Opplysning::ErOver18Aar]),
            ..Default::default()
        }
        .build();

        let avvist_fra_veileder: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            bruker_type: BrukerType::Veileder,
            utfoert_av_id: "Z991459".to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();

        let avvist_under_18: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();

        let meldinger = [
            irrelevant_hendelse.as_json(),
            avvist_fra_veileder.as_json(),
            avvist_under_18.as_json(),
            avvist_under_18.as_json(),
        ];

        for msg in meldinger {
            let mut tx = pg_pool.begin().await?;
            process_hendelselogg_message(msg.as_bytes(), &app_config, &mut tx).await?;
            tx.commit().await?;
        }

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .unwrap();

        assert_eq!(oppgave.type_, OppgaveType::AvvistUnder18);
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);
        assert_eq!(oppgave.hendelse_logg.len(), 2);
        assert!(
            contains_all(
                &oppgave.opplysninger,
                &[ErUnder18Aar.to_string(), BosattEtterFregLoven.to_string()]
            ),
            "Mangler forventede opplysninger: {:?}",
            oppgave.opplysninger
        );
        assert_eq!(oppgave.arbeidssoeker_id, ARB_ID);
        assert_eq!(oppgave.identitetsnummer, IDENT);

        Ok(())
    }

    #[tokio::test]
    async fn test_gammel_hendelse_gir_ignorert_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut app_config = read_application_config()?;
        app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt =
            rfc3339("2030-01-01T00:00:00Z").into();

        let avvist: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();
        let message = avvist.as_json();

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .unwrap();
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
        app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt =
            rfc3339("2030-01-01T00:00:00Z").into();

        let avvist: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();
        let message = avvist.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, OppgaveStatus::Ignorert);
        tx.commit().await?;

        let app_config = read_application_config()?;
        let avvist_2: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();
        let message_2 = avvist_2.as_json();

        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message_2.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .unwrap();
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

        let avvist_1: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();
        let message_1 = avvist_1.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message_1.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .unwrap();
        bytt_oppgave_status(
            oppgave.id,
            OppgaveStatus::Ubehandlet,
            Ferdigbehandlet,
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        let avvist_2: Avvist = AvvistBuilder {
            arbeidssoeker_id: ARB_ID,
            identitetsnummer: IDENT.to_string(),
            opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
            ..Default::default()
        }
        .build();
        let message_2 = avvist_2.as_json();
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(message_2.as_bytes(), &app_config, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(ARB_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .unwrap();
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);
        assert_eq!(
            oppgave.hendelse_logg[0].status,
            HendelseLoggStatus::OppgaveOpprettet
        );

        Ok(())
    }
}
