use crate::domain::oppgave_status::OppgaveStatus;
use anyhow::Result;
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{FromRow, Postgres, Transaction};
use std::sync::LazyLock;
use strum::IntoEnumIterator;

static OPPGAVER_PER_STATUS: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "avvist_til_oppgave_oppgaver_total",
        "Antall oppgaver per status",
        &["status"]
    )
    .expect("Failed to register avvist_til_oppgave_oppgaver_total gauge")
});

pub async fn oppdater(transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let rader = hent_antall(transaction).await?;
    for oppgave_status in OppgaveStatus::iter() {
        let antall = rader
            .iter()
            .find(|rad| rad.status == oppgave_status.to_string())
            .map(|rad| rad.antall)
            .unwrap_or(0);
        OPPGAVER_PER_STATUS
            .with_label_values(&[&oppgave_status.to_string()])
            .set(antall as f64);
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct OppgaveStatusAntall {
    status: String,
    antall: i64,
}

async fn hent_antall(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<OppgaveStatusAntall>> {
    let rader = sqlx::query_as::<_, OppgaveStatusAntall>(
        r#"
        SELECT status, COUNT(*) as antall
        FROM oppgaver
        GROUP BY status
        "#,
    )
    .fetch_all(&mut **transaction)
    .await?;

    Ok(rader)
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
            .find(|rad| rad.status == Ubehandlet.to_string())
            .map(|rad| rad.antall);
        let ferdigbehandlet_antall = oppgave_status_antall
            .iter()
            .find(|rad| rad.status == Ferdigbehandlet.to_string())
            .map(|rad| rad.antall);

        assert_eq!(ubehandlet_antall, Some(2));
        assert_eq!(ferdigbehandlet_antall, Some(1));
        assert_eq!(oppgave_status_antall.len(), 2);

        Ok(())
    }
}
