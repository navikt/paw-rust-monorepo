use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_row;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::kriterier::vurder_oppholdsstatus;
use crate::domain::oppgave_status::OppgaveStatus;
use chrono::Utc;
use interne_hendelser::Startet;
use sqlx::{Postgres, Transaction};
use OppgaveStatus::{Ferdigbehandlet, Ubehandlet};
use types::arbeidssoeker_id::ArbeidssoekerId;
use crate::metrics;

pub async fn opprett_vurder_oppholdsstatus_oppgave(
    startet_hendelse: &Startet,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    if vurder_oppholdsstatus::KRITERIER.ikke_oppfylt_av(startet_hendelse) {
        return Ok(());
    }

    let oppgave_type = vurder_oppholdsstatus::KRITERIER.oppgave_type;
    metrics::kriterier_oppfylt::inkrement(oppgave_type);

    let arbeidssoeker_id = ArbeidssoekerId::from(startet_hendelse.id);
    let eksisterende_oppgave = hent_nyeste_oppgave(arbeidssoeker_id, oppgave_type, tx).await?;
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

    let oppgave_row = to_oppgave_row(startet_hendelse, oppgave_type, Ubehandlet);
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

