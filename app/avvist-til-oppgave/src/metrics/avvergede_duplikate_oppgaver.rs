use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use anyhow::Result;
use chrono::{TimeZone, Utc};
use prometheus::{register_gauge, Gauge};
use sqlx::{Postgres, Transaction};
use std::sync::OnceLock;

static DUPLIKATE_OPPGAVER_AVVERGET: OnceLock<Gauge> = OnceLock::new();

pub async fn oppdater(transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let antall = hent_antall_duplikater_avverget(transaction).await?;
    sett_forhindrede_duplikater(antall);
    Ok(())
}

async fn hent_antall_duplikater_avverget(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64> {
    // Teller fra 10. mars 2026 — data før dette er upålitelig pga. en bug
    let fra_tidspunkt = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
    let antall: i64 = sqlx::query_scalar(
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

fn sett_forhindrede_duplikater(antall: i64) {
    DUPLIKATE_OPPGAVER_AVVERGET
        .get_or_init(|| {
            register_gauge!(
                "avvist_til_oppgave_forhindrede_duplikater_total",
                "Antall ganger en arbeidssøker ble avvist på nytt mens en aktiv oppgave allerede fantes (OPPGAVE_FINNES_ALLEREDE)"
            )
            .expect("Failed to register avvist_til_oppgave_forhindrede_duplikater_total gauge")
        })
        .set(antall as f64);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{insert_oppgave, insert_oppgave_hendelse_logg};
    use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use anyhow::Result;
    use chrono::Duration;
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
        let antall = hent_antall_duplikater_avverget(&mut tx).await?;

        assert_eq!(
            antall, 1,
            "Skal kun telle OPPGAVE_FINNES_ALLEREDE etter cutoff"
        );

        Ok(())
    }
}
