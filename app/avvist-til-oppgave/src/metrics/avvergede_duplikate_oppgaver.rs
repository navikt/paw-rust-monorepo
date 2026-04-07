use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use anyhow::Result;
use chrono::{DateTime, Utc};
use prometheus::{register_gauge, Gauge};
use sqlx::{Postgres, Transaction};
use std::sync::LazyLock;

static DUPLIKATE_OPPGAVER_AVVERGET: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "avvist_til_oppgave_forhindrede_duplikater_total",
        "Antall ganger en arbeidssøker ble avvist på nytt mens en aktiv oppgave allerede fantes (OPPGAVE_FINNES_ALLEREDE)"
    )
    .expect("Failed to register avvist_til_oppgave_forhindrede_duplikater_total gauge")
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
        FROM oppgave_hendelse_logg
        WHERE status = $1
          AND tidspunkt >= $2
        "#,
    )
    .bind(OppgaveFinnesAllerede.to_string())
    .bind(fra_tidspunkt)
    .fetch_one(&mut **transaction)
    .await?;

    Ok(antall)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{insert_oppgave, insert_oppgave_hendelse_logg};
    use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use anyhow::Result;
    use chrono::{Duration, TimeZone, Utc};
    use paw_test::setup_test_db::setup_test_db;
    use HendelseLoggStatus::OppgaveOpprettet;

    #[tokio::test]
    async fn test_hent_antall_duplikate_oppgaver() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let oppgave_id = insert_oppgave(&InsertOppgaveRow::default(), &mut tx).await?;
        let etter_cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap() + Duration::days(1);
        let foer_cutoff = Utc.with_ymd_and_hms(2026, 3, 9, 0, 0, 0).unwrap();

        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: OppgaveFinnesAllerede.to_string(),
                melding: String::new(),
                tidspunkt: etter_cutoff,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: OppgaveFinnesAllerede.to_string(),
                melding: String::new(),
                tidspunkt: foer_cutoff,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: OppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: etter_cutoff,
            },
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let antall = hent_antall_duplikater_avverget(cutoff, &mut tx).await?;

        assert_eq!(
            antall, 1,
            "Skal kun telle OPPGAVE_FINNES_ALLEREDE etter cutoff"
        );

        Ok(())
    }
}
