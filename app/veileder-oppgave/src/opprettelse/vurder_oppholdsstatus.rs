use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, lagre_oppgave, oppdater_hendelse_logg,
};
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
        let hendelse_logg = HendelseLoggEntry::new(
            HendelseLoggStatus::OppgaveFinnesAllerede,
            "Arbeidssøkeren har allerede en aktiv vurder oppholdsstatus oppgave".to_string(),
            Utc::now(),
        );
        oppdater_hendelse_logg(oppgave.id(), hendelse_logg, tx).await?;
        return Ok(());
    }

    let identitetsnummer = Identitetsnummer::new(startet_hendelse.identitetsnummer().to_string())
        .expect("Ugyldig identitetsnummer i Kafka-hendelse som oppfyller kriteriene for vurder oppholdsstatus");

    let oppgave = Oppgave::new(
        startet_hendelse.hendelse_id(),
        oppgave_type,
        Ubehandlet,
        hent_opplysninger_fra(startet_hendelse),
        arbeidssoeker_id,
        identitetsnummer,
        startet_hendelse.metadata().tidspunkt,
    );

    let oppgave_id = lagre_oppgave(&oppgave, tx).await?;

    let hendelse_logg = HendelseLoggEntry::new(
        HendelseLoggStatus::OppgaveOpprettet,
        "Oppretter vurder oppholdsstatus oppgave".to_string(),
        oppgave.tidspunkt,
    );
    oppdater_hendelse_logg(oppgave_id, hendelse_logg, tx).await?;

    Ok(())
}

fn hent_opplysninger_fra(startet_hendelse: &Startet) -> Vec<String> {
    startet_hendelse
        .opplysninger()
        .iter()
        .map(|opplysning| opplysning.to_string())
        .collect()
}

