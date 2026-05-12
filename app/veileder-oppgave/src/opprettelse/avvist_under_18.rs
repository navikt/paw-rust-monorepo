use crate::config::ApplicationConfig;
use crate::db::oppgave_functions::{
    hent_nyeste_oppgave, insert_oppgave, insert_oppgave_hendelse_logg,
};
use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
use crate::db::oppgave_row::to_oppgave_insert_row;
use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::kriterier::avvist_under_18;
use crate::domain::oppgave::Oppgave;
use crate::domain::oppgave_status::OppgaveStatus;
use chrono::Utc;
use interne_hendelser::Avvist;
use interne_hendelser::Hendelse;
use sqlx::{Postgres, Transaction};
use OppgaveStatus::{Ferdigbehandlet, Ignorert, Ubehandlet};
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::identitetsnummer::Identitetsnummer;
use crate::metrics;

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

    let oppgave_type = avvist_under_18::KRITERIER.oppgave_type;
    metrics::kriterier_oppfylt::inkrement(oppgave_type);

    let arbeidssoeker_id = ArbeidssoekerId::from(avvist_hendelse.id);
    let identitetsnummer = Identitetsnummer::new(avvist_hendelse.identitetsnummer().to_string())
        .expect("Ugyldig identitetsnummer i Kafka-hendelse — avviser");
    let opplysninger = hent_opplysninger_fra(avvist_hendelse);

    if avvist_hendelse.metadata.tidspunkt >= opprett_avvist_under_18_oppgaver_fra_tidspunkt {
        let eksisterende_oppgave = hent_nyeste_oppgave(arbeidssoeker_id, oppgave_type, tx).await?;
        if let Some(oppgave) = &eksisterende_oppgave
            && oppgave.status != Ferdigbehandlet
            && oppgave.status != Ignorert
        {
            let hendelse_logg_entry = HendelseLoggEntry::new(
                HendelseLoggStatus::OppgaveFinnesAllerede,
                "Arbeidssøkeren har allerede en aktiv oppgave for avvist registrering".to_string(),
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

        let oppgave = Oppgave::new(
            oppgave_type,
            Ubehandlet,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            avvist_hendelse.metadata.tidspunkt,
        );

        let oppgave_row = to_oppgave_insert_row(&oppgave, avvist_hendelse.hendelse_id());
        let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;

        let hendelse_logg_entry = HendelseLoggEntry::new(
            HendelseLoggStatus::OppgaveOpprettet,
            "Oppretter oppgave for avvist registrering".to_string(),
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
    } else {
        let oppgave = Oppgave::new(
            oppgave_type,
            Ignorert,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            avvist_hendelse.metadata.tidspunkt,
        );

        let oppgave_row = to_oppgave_insert_row(&oppgave, avvist_hendelse.hendelse_id());
        let oppgave_id = insert_oppgave(&oppgave_row, tx).await?;

        let hendelse_logg_entry = HendelseLoggEntry::new(
            HendelseLoggStatus::OppgaveIgnorert,
            format!(
                "Oppretter oppgave for avvist registrering med status {} fordi hendelse er eldre enn {}",
                Ignorert,
                opprett_avvist_under_18_oppgaver_fra_tidspunkt
            ),
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
    }

    Ok(())
}

fn hent_opplysninger_fra(avvist_hendelse: &Avvist) -> Vec<String> {
    avvist_hendelse
        .opplysninger()
        .iter()
        .map(|opplysning| opplysning.to_string())
        .collect()
}

