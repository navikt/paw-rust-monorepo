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
use crate::metrics::kriterier_oppfylt;
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

    if avvist_under_18::KRITERIER.ikke_oppfylt_av(avvist_hendelse) {
        return Ok(());
    }

    let oppgave_type: OppgaveType = avvist_under_18::KRITERIER.oppgave_type;
    kriterier_oppfylt::inkrement(oppgave_type);

    if avvist_hendelse.metadata.tidspunkt >= opprett_avvist_under_18_oppgaver_fra_tidspunkt {
        let arbeidssoeker_id = avvist_hendelse.id;
        let eksisterende_oppgave = hent_nyeste_oppgave(arbeidssoeker_id, oppgave_type, tx).await?;
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
            oppgave_type,
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
        let oppgave_row = to_oppgave_row(avvist_hendelse, oppgave_type, Ignorert);
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

