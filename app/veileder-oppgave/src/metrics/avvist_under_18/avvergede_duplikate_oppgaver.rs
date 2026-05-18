use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use crate::domain::oppgave_type::OppgaveType;
use anyhow::Result;
use chrono::{DateTime, Utc};
use prometheus::{register_gauge, Gauge};
use sqlx::{Postgres, Transaction};
use std::sync::LazyLock;

static DUPLIKATE_OPPGAVER_AVVERGET: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "veileder_oppgave_forhindrede_duplikater_total",
        "Antall ganger en arbeidssøker ble avvist på nytt mens en aktiv oppgave allerede fantes (OPPGAVE_FINNES_ALLEREDE)"
    )
    .expect("Failed to register veileder_oppgave_forhindrede_duplikater_total gauge")
});

pub async fn oppdater(fra_tidspunkt: DateTime<Utc>, transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let antall = hent_antall_duplikater_avverget(fra_tidspunkt, transaction).await?;
    DUPLIKATE_OPPGAVER_AVVERGET.set(antall as f64);
    Ok(())
}

async fn hent_antall_duplikater_avverget(
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64> {
    let antall: i64 = sqlx::query_scalar(
        //language=PostgreSQL
        r#"
        SELECT COUNT(*)
        FROM oppgave_hendelse_logg ohl
        JOIN oppgaver o ON o.id = ohl.oppgave_id
        WHERE ohl.status = $1
          AND ohl.tidspunkt >= $2
          AND o.type = $3
        "#,
    )
    .bind(OppgaveFinnesAllerede.to_string())
    .bind(fra_tidspunkt)
    .bind(OppgaveType::AvvistUnder18.to_string())
    .fetch_one(&mut **transaction)
    .await?;

    Ok(antall)
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
    use chrono::{Duration, TimeZone, Utc};
    use paw_test::setup_test_db::setup_test_db;
    use types::arbeidssoeker_id::ArbeidssoekerId;
    use types::identitetsnummer::Identitetsnummer;
    use uuid::Uuid;
    use HendelseLoggStatus::OppgaveOpprettet;

    #[tokio::test]
    async fn test_hent_antall_duplikate_oppgaver() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let etter_cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap() + Duration::days(1);
        let foer_cutoff = Utc.with_ymd_and_hms(2026, 3, 9, 0, 0, 0).unwrap();

        let avvist_oppgave = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(1), Identitetsnummer::new("12345678901".to_string()).unwrap(), Utc::now());
        let avvist_oppgave_id = lagre_oppgave(&avvist_oppgave, &mut tx).await?;
        oppdater_hendelse_logg(avvist_oppgave_id, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), etter_cutoff), &mut tx).await?;
        oppdater_hendelse_logg(avvist_oppgave_id, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), foer_cutoff), &mut tx).await?;
        oppdater_hendelse_logg(avvist_oppgave_id, HendelseLoggEntry::new(OppgaveOpprettet, String::new(), etter_cutoff), &mut tx).await?;

        // VurderOppholdsstatus med duplikat — skal IKKE telles
        let vurder_oppgave_ignorert = Oppgave::new(Uuid::new_v4(), VurderOppholdsstatus, Ubehandlet, vec![], ArbeidssoekerId(2), Identitetsnummer::new("12345678902".to_string()).unwrap(), Utc::now());
        let vurder_oppgave_id = lagre_oppgave(&vurder_oppgave_ignorert, &mut tx).await?;
        oppdater_hendelse_logg(vurder_oppgave_id, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), etter_cutoff), &mut tx).await?;

        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let antall = hent_antall_duplikater_avverget(cutoff, &mut tx).await?;

        assert_eq!(
            antall, 1,
            "Skal kun telle OPPGAVE_FINNES_ALLEREDE for AVVIST_UNDER_18 etter cutoff"
        );

        Ok(())
    }
}
