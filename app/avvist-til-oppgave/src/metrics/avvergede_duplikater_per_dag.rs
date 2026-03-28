use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use anyhow::Result;
use chrono::{TimeZone, Utc};
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{FromRow, Postgres, Transaction};
use std::sync::OnceLock;

static AVVERGEDE_DUPLIKATER_PER_DAG: OnceLock<GaugeVec> = OnceLock::new();

pub async fn oppdater(transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let avvergede_duplikater_per_dag = hent_avvergede_duplikater_per_dag(transaction).await?;
    sett_avvergede_duplikater_per_dag(&avvergede_duplikater_per_dag);
    Ok(())
}

#[derive(Debug, FromRow)]
struct AvvergedeDuplikaterPerDag {
    dato: String,
    antall: i64,
}

async fn hent_avvergede_duplikater_per_dag(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<AvvergedeDuplikaterPerDag>> {
    // Teller fra 10. mars 2026 — data før dette er upålitelig pga. en bug
    let fra_tidspunkt = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
    let vis_siste_dager: i64 = 30;
    let rader = sqlx::query_as::<_, AvvergedeDuplikaterPerDag>(
        r#"
        SELECT TO_CHAR(tidspunkt, 'DD.MM') AS dato, COUNT(*) AS antall
        FROM oppgave_hendelse_logg
        WHERE status = $1
          AND tidspunkt >= $2
        GROUP BY dato
        ORDER BY dato DESC
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

fn sett_avvergede_duplikater_per_dag(rader: &[AvvergedeDuplikaterPerDag]) {
    let gauge = AVVERGEDE_DUPLIKATER_PER_DAG.get_or_init(|| {
        register_gauge_vec!(
            "avvist_til_oppgave_avvergede_duplikater_per_dag",
            "Antall duplikate oppgaver avverget per dag (OPPGAVE_FINNES_ALLEREDE)",
            &["dato"]
        )
        .expect("Failed to register avvist_til_oppgave_avvergede_duplikater_per_dag gauge")
    });

    gauge.reset();
    for rad in rader {
        gauge.with_label_values(&[&rad.dato]).set(rad.antall as f64);
    }
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
        let rader = hent_avvergede_duplikater_per_dag(&mut tx).await?;

        assert_eq!(rader.len(), 2, "Skal ha to datoer etter cutoff");
        let avvergede_duplikater_forste_dag = rader
            .iter()
            .find(|avverget_duplikat| avverget_duplikat.dato == "15.03")
            .unwrap();
        let avvergede_duplikater_andre_dag = rader
            .iter()
            .find(|avverget_duplikat| avverget_duplikat.dato == "16.03")
            .unwrap();
        assert_eq!(avvergede_duplikater_forste_dag.antall, 2);
        assert_eq!(avvergede_duplikater_andre_dag.antall, 1);

        Ok(())
    }
}
