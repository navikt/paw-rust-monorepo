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

