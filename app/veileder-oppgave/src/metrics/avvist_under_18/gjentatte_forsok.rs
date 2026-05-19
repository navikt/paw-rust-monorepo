use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use crate::domain::oppgave_type::OppgaveType;
use anyhow::Result;
use chrono::{DateTime, Utc};
use prometheus::{register_gauge, Gauge};
use sqlx::{Postgres, Transaction};
use std::sync::LazyLock;

static GJENTATTE_FORSOK_GJENNOMSNITT: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "veileder_oppgave_gjentatte_forsok_gjennomsnitt",
        "Gjennomsnittlig antall ekstra registreringsforsøk per arbeidssøker under 18 etter første avvisning"
    )
    .expect("Failed to register veileder_oppgave_gjentatte_forsok_gjennomsnitt gauge")
});

pub async fn oppdater(fra_tidspunkt: DateTime<Utc>, transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let gjennomsnitt = hent_gjentatte_forsok_gjennomsnitt(fra_tidspunkt, transaction).await?;
    GJENTATTE_FORSOK_GJENNOMSNITT.set(gjennomsnitt);
    Ok(())
}

async fn hent_gjentatte_forsok_gjennomsnitt(
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<f64> {
    let gjennomsnitt: Option<f64> = sqlx::query_scalar(
        //language=PostgreSQL
        r#"
        SELECT CAST(AVG(antall_forsok) AS FLOAT8)
        FROM (
            SELECT o.identitetsnummer, COUNT(ohl.id) AS antall_forsok
            FROM oppgaver o
            LEFT JOIN oppgave_hendelse_logg ohl
                ON ohl.oppgave_id = o.id
                AND ohl.status = $1
            WHERE o.tidspunkt >= $2
              AND o.type = $3
            GROUP BY o.identitetsnummer
        ) AS antall_forsok_per_person
        "#,
    )
    .bind(OppgaveFinnesAllerede.to_string())
    .bind(fra_tidspunkt)
    .bind(OppgaveType::AvvistUnder18.to_string())
    .fetch_one(&mut **transaction)
    .await?;

    Ok(gjennomsnitt.unwrap_or(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{lagre_oppgave, oppdater_hendelse_logg};
    use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use crate::domain::oppgave::Oppgave;
    use crate::domain::oppgave_status::OppgaveStatus::Ubehandlet;
    use crate::domain::oppgave_type::OppgaveType::{AvvistUnder18, VurderOppholdsstatus};
    use anyhow::Result;
    use chrono::{TimeZone, Utc};
    use paw_test::setup_test_db::setup_test_db;
    use types::arbeidssoeker_id::ArbeidssoekerId;
    use types::identitetsnummer::Identitetsnummer;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_hent_gjentatte_forsok_gjennomsnitt() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let tidspunkt_etter_cutoff = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
        let tidspunkt_foer_cutoff = Utc.with_ymd_and_hms(2026, 3, 9, 0, 0, 0).unwrap();

        // Person 1: to ekstra forsøk etter cutoff (AvvistUnder18)
        let avvist_med_to_forsok = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(1), Identitetsnummer::new("12345678901".to_string()).unwrap(), tidspunkt_etter_cutoff);
        let oppgave_id_1 = lagre_oppgave(&avvist_med_to_forsok, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_1, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), tidspunkt_etter_cutoff), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_1, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), tidspunkt_etter_cutoff), &mut tx).await?;

        // Person 2: null ekstra forsøk (AvvistUnder18)
        let avvist_uten_forsok = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(2), Identitetsnummer::new("12345678902".to_string()).unwrap(), tidspunkt_etter_cutoff);
        let oppgave_id_2 = lagre_oppgave(&avvist_uten_forsok, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_2, HendelseLoggEntry::new(HendelseLoggStatus::OppgaveOpprettet, String::new(), tidspunkt_etter_cutoff), &mut tx).await?;

        // Person 3: VurderOppholdsstatus med forsøk — skal IKKE telles
        let vurder_oppgave = Oppgave::new(Uuid::new_v4(), VurderOppholdsstatus, Ubehandlet, vec![], ArbeidssoekerId(3), Identitetsnummer::new("12345678905".to_string()).unwrap(), tidspunkt_etter_cutoff);
        let oppgave_id_vurder = lagre_oppgave(&vurder_oppgave, &mut tx).await?;
        for _ in 0..5 {
            oppdater_hendelse_logg(oppgave_id_vurder, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), tidspunkt_etter_cutoff), &mut tx).await?;
        }

        // Person 4: oppgave før cutoff — skal ikke telles
        let avvist_foer_cutoff = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(4), Identitetsnummer::new("12345678903".to_string()).unwrap(), tidspunkt_foer_cutoff);
        let oppgave_id_3 = lagre_oppgave(&avvist_foer_cutoff, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_3, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), tidspunkt_foer_cutoff), &mut tx).await?;

        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let gjennomsnitt = hent_gjentatte_forsok_gjennomsnitt(cutoff, &mut tx).await?;

        // Person 1: 2 forsøk, person 2: 0 forsøk → gjennomsnitt = 1.0 (VurderOppholdsstatus filtreres bort)
        assert_eq!(gjennomsnitt, 1.0);

        Ok(())
    }
}
