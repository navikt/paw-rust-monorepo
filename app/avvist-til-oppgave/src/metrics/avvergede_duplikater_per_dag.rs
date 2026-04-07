use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use anyhow::Result;
use chrono::{DateTime, Utc};
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{FromRow, Postgres, Transaction};
use std::sync::LazyLock;

static AVVERGEDE_DUPLIKATER_PER_DAG: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "avvist_til_oppgave_avvergede_duplikater_per_dag",
        "Antall duplikate oppgaver avverget per dag (OPPGAVE_FINNES_ALLEREDE)",
        &["dato"]
    )
    .expect("Failed to register avvist_til_oppgave_avvergede_duplikater_per_dag gauge")
});

pub async fn oppdater(fra_tidspunkt: DateTime<Utc>, transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let rader = hent_avvergede_duplikater_per_dag(fra_tidspunkt, transaction).await?;
    AVVERGEDE_DUPLIKATER_PER_DAG.reset();
    for rad in &rader {
        AVVERGEDE_DUPLIKATER_PER_DAG
            .with_label_values(&[&rad.dato])
            .set(rad.antall as f64);
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct AvvergedeDuplikaterPerDag {
    dato: String,
    antall: i64,
}

async fn hent_avvergede_duplikater_per_dag(
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<AvvergedeDuplikaterPerDag>> {
    let vis_siste_dager: i64 = 30;
    let rader = sqlx::query_as::<_, AvvergedeDuplikaterPerDag>(
        r#"
        SELECT TO_CHAR(DATE(tidspunkt), 'YYYY-MM-DD') AS dato, COUNT(*) AS antall
        FROM oppgave_hendelse_logg
        WHERE status = $1
          AND tidspunkt >= $2
        GROUP BY DATE(tidspunkt)
        ORDER BY DATE(tidspunkt) DESC
        LIMIT $3
        "#,
    )
    .bind(OppgaveFinnesAllerede.to_string())
    .bind(fra_tidspunkt)
    .bind(vis_siste_dager)
    .fetch_all(&mut **transaction)
    .await?;

    Ok(rader)
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

    #[tokio::test]
    async fn test_hent_avvergede_duplikater_per_dag() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let oppgave_id = insert_oppgave(&InsertOppgaveRow::default(), &mut tx).await?;
        let første_dag = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
        let andre_dag = første_dag + Duration::days(1);
        let tidspunkt_før_cutoff = Utc.with_ymd_and_hms(2026, 3, 9, 0, 0, 0).unwrap();

        // forste_dag_med_duplikat: to duplikater (verifiserer at telleren blir 2, ikke 1)
        // andre_dag_med_duplikat: ett duplikat
        // tidspunkt_foer_cutoff: skal ikke telles
        for tidspunkt in [første_dag, første_dag, andre_dag, tidspunkt_før_cutoff] {
            insert_oppgave_hendelse_logg(
                &InsertOppgaveHendelseLoggRow {
                    oppgave_id,
                    status: OppgaveFinnesAllerede.to_string(),
                    melding: String::new(),
                    tidspunkt,
                },
                &mut tx,
            )
            .await?;
        }
        // Annen status — skal ikke telles
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id,
                status: HendelseLoggStatus::OppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: første_dag,
            },
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let rader = hent_avvergede_duplikater_per_dag(cutoff, &mut tx).await?;

        assert_eq!(rader.len(), 2, "Skal ha to datoer etter cutoff");
        let avvergede_duplikater_forste_dag = rader
            .iter()
            .find(|avverget_duplikat| avverget_duplikat.dato == "2026-03-15")
            .unwrap();
        let avvergede_duplikater_andre_dag = rader
            .iter()
            .find(|avverget_duplikat| avverget_duplikat.dato == "2026-03-16")
            .unwrap();
        assert_eq!(avvergede_duplikater_forste_dag.antall, 2);
        assert_eq!(avvergede_duplikater_andre_dag.antall, 1);

        Ok(())
    }
}
