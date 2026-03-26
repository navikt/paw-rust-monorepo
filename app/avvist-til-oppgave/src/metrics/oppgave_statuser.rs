use crate::domain::oppgave_status::OppgaveStatus;
use anyhow::Result;
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{Postgres, Transaction};
use std::sync::OnceLock;
use strum::IntoEnumIterator;

static OPPGAVER_PER_STATUS: OnceLock<GaugeVec> = OnceLock::new();

pub async fn oppdater(transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let oppgave_status_antall = hent_antall(transaction).await?;
    sett_oppgave_statuser(&oppgave_status_antall);
    Ok(())
}

struct OppgaveStatusAntall {
    oppgave_status: OppgaveStatus,
    antall: i64,
}

async fn hent_antall(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<OppgaveStatusAntall>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        r#"
        SELECT status, COUNT(*) as antall
        FROM oppgaver
        GROUP BY status
        "#,
    )
    .fetch_all(&mut **transaction)
    .await?;

    let result = rows
        .into_iter()
        .filter_map(|(status_str, antall)| {
            status_str
                .parse::<OppgaveStatus>()
                .ok()
                .map(|oppgave_status| OppgaveStatusAntall {
                    oppgave_status,
                    antall,
                })
        })
        .collect();

    Ok(result)
}

fn sett_oppgave_statuser(oppgave_status_antall: &[OppgaveStatusAntall]) {
    let gauge = OPPGAVER_PER_STATUS.get_or_init(|| {
        register_gauge_vec!(
            "avvist_til_oppgave_oppgaver_total",
            "Antall oppgaver per status",
            &["status"]
        )
        .expect("Failed to register avvist_til_oppgave_oppgaver_total gauge")
    });

    for oppgave_status in OppgaveStatus::iter() {
        let antall = oppgave_status_antall
            .iter()
            .find(|entry| entry.oppgave_status == oppgave_status)
            .map(|entry| entry.antall)
            .unwrap_or(0);
        gauge
            .with_label_values(&[&oppgave_status.to_string()])
            .set(antall as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::insert_oppgave;
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::oppgave_status::OppgaveStatus::{Ferdigbehandlet, Ubehandlet};
    use anyhow::Result;
    use paw_test::setup_test_db::setup_test_db;

    #[tokio::test]
    async fn test_hent_antall_oppgaver_per_status() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        insert_oppgave(
            &InsertOppgaveRow {
                status: Ubehandlet.to_string(),
                arbeidssoeker_id: 1,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave(
            &InsertOppgaveRow {
                status: Ubehandlet.to_string(),
                arbeidssoeker_id: 2,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave(
            &InsertOppgaveRow {
                status: Ferdigbehandlet.to_string(),
                arbeidssoeker_id: 3,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave_status_antall = hent_antall(&mut tx).await?;

        let ubehandlet_antall = oppgave_status_antall
            .iter()
            .find(|entry| entry.oppgave_status == Ubehandlet)
            .map(|entry| entry.antall);
        let ferdigbehandlet_antall = oppgave_status_antall
            .iter()
            .find(|entry| entry.oppgave_status == Ferdigbehandlet)
            .map(|entry| entry.antall);

        assert_eq!(ubehandlet_antall, Some(2));
        assert_eq!(ferdigbehandlet_antall, Some(1));
        assert_eq!(oppgave_status_antall.len(), 2);

        Ok(())
    }
}
