use crate::domain::oppgave_status::OppgaveStatus;
use crate::domain::oppgave_type::OppgaveType;
use anyhow::Result;
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{FromRow, Postgres, Transaction};
use std::sync::LazyLock;
use strum::IntoEnumIterator;

static OPPGAVER_PER_STATUS: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "veileder_oppgave_oppgaver_total",
        "Antall oppgaver per status og type",
        &["status", "type"]
    )
    .expect("Failed to register veileder_oppgave_oppgaver_total gauge")
});

pub async fn oppdater(transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let rader = hent_antall(transaction).await?;
    for oppgave_status in OppgaveStatus::iter() {
        for oppgave_type in OppgaveType::iter() {
            let antall = rader
                .iter()
                .find(|rad| {
                    rad.status == oppgave_status.to_string()
                        && rad.type_ == oppgave_type.to_string()
                })
                .map(|rad| rad.antall)
                .unwrap_or(0);
            OPPGAVER_PER_STATUS
                .with_label_values(&[&oppgave_status.to_string(), &oppgave_type.to_string()])
                .set(antall as f64);
        }
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct OppgaveStatusAntall {
    status: String,
    #[sqlx(rename = "type")]
    type_: String,
    antall: i64,
}

async fn hent_antall(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<OppgaveStatusAntall>> {
    let rader = sqlx::query_as::<_, OppgaveStatusAntall>(
        r#"
        SELECT status, type, COUNT(*) as antall
        FROM oppgaver
        GROUP BY status, type
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
    use crate::domain::oppgave_type::OppgaveType::{AvvistUnder18, VurderOpphold};
    use anyhow::Result;
    use paw_test::setup_test_db::setup_test_db;

    #[tokio::test]
    async fn test_hent_antall_oppgaver_per_status_og_type() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        insert_oppgave(
            &InsertOppgaveRow {
                type_: AvvistUnder18.to_string(),
                status: Ubehandlet.to_string(),
                arbeidssoeker_id: 1,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave(
            &InsertOppgaveRow {
                type_: AvvistUnder18.to_string(),
                status: Ubehandlet.to_string(),
                arbeidssoeker_id: 2,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave(
            &InsertOppgaveRow {
                type_: AvvistUnder18.to_string(),
                status: Ferdigbehandlet.to_string(),
                arbeidssoeker_id: 3,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave(
            &InsertOppgaveRow {
                type_: VurderOpphold.to_string(),
                status: Ubehandlet.to_string(),
                arbeidssoeker_id: 4,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let rader = hent_antall(&mut tx).await?;

        let avvist_ubehandlet = rader
            .iter()
            .find(|r| r.status == Ubehandlet.to_string() && r.type_ == AvvistUnder18.to_string())
            .map(|r| r.antall);
        let avvist_ferdigbehandlet = rader
            .iter()
            .find(|r| r.status == Ferdigbehandlet.to_string() && r.type_ == AvvistUnder18.to_string())
            .map(|r| r.antall);
        let vurder_ubehandlet = rader
            .iter()
            .find(|r| r.status == Ubehandlet.to_string() && r.type_ == VurderOpphold.to_string())
            .map(|r| r.antall);

        assert_eq!(avvist_ubehandlet, Some(2));
        assert_eq!(avvist_ferdigbehandlet, Some(1));
        assert_eq!(vurder_ubehandlet, Some(1));
        assert_eq!(rader.len(), 3);

        Ok(())
    }
}
