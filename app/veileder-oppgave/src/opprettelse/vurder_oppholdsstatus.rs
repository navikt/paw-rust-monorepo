use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_insert_row;
use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::kriterier::vurder_oppholdsstatus;
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus;
use chrono::Utc;
use interne_hendelser::Hendelse;
use interne_hendelser::Startet;
use sqlx::{Postgres, Transaction};
use OppgaveStatus::{Ferdigbehandlet, Ubehandlet};
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;
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
        let hendelse_logg_entry = HendelseLoggEntry::new(
            HendelseLoggStatus::OppgaveFinnesAllerede,
            "Arbeidssøkeren har allerede en aktiv vurder oppholdsstatus oppgave".to_string(),
            Utc::now(),
        );
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave.id(),
                status: hendelse_logg_entry.status.to_string(),
                melding: hendelse_logg_entry.melding,
                tidspunkt: hendelse_logg_entry.tidspunkt,
            },
            tx,
        )
        .await?;
        return Ok(());
    }

    let identitetsnummer = Identitetsnummer::new(startet_hendelse.identitetsnummer().to_string())
        .expect("Ugyldig identitetsnummer i Kafka-hendelse — avviser");

    let oppgave = Oppgave::new(
        oppgave_type,
        Ubehandlet,
        hent_opplysninger_fra(startet_hendelse),
        arbeidssoeker_id,
        identitetsnummer,
        startet_hendelse.metadata().tidspunkt,
    );

    let oppgave_row = to_oppgave_insert_row(&oppgave, startet_hendelse.hendelse_id());
    let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;

    let hendelse_logg_entry = HendelseLoggEntry::new(
        HendelseLoggStatus::OppgaveOpprettet,
        "Oppretter vurder oppholdsstatus oppgave".to_string(),
        oppgave.tidspunkt,
    );
    insert_oppgave_hendelse_logg(
        &InsertOppgaveHendelseLoggRow {
            oppgave_id,
            status: hendelse_logg_entry.status.to_string(),
            melding: hendelse_logg_entry.melding,
            tidspunkt: hendelse_logg_entry.tidspunkt,
        },
        tx,
    )
    .await?;

    Ok(())
}

fn hent_opplysninger_fra(startet_hendelse: &Startet) -> Vec<String> {
    startet_hendelse
        .opplysninger()
        .iter()
        .map(|opplysning| opplysning.to_string())
        .collect()
}

