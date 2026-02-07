use crate::db::oppgave_row::OppgaveRow;
use crate::db::oppgave_status_logg_row::OppgaveStatusLoggRow;
use crate::domain::oppgave_status::OppgaveStatus;
use sqlx::{Postgres, Transaction};
use std::error::Error;

pub async fn insert_oppgave_med(
    oppgave_status: OppgaveStatus,
    oppgave_row: &OppgaveRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64, Box<dyn Error>> {
    let oppgave_id = insert_oppgave(oppgave_row, transaction).await?;

    let status_logg_row = OppgaveStatusLoggRow {
        oppgave_id,
        status: oppgave_status.to_string(),
        melding: "Ubehandlet oppgave opprettet".to_string(),
        tidspunkt: oppgave_row.tidspunkt.clone(),
    };

    insert_oppgave_status_logg(&status_logg_row, transaction).await?;

    Ok(oppgave_id)
}

async fn insert_oppgave(
    oppgave_row: &OppgaveRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64, Box<dyn Error>> {
    let oppgave_id = sqlx::query_scalar(
        r#"
        INSERT INTO oppgaver (type, melding_id, opplysninger, arbeidssoeker_id, identitetsnummer, tidspunkt)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
        .bind(&oppgave_row.type_)
        .bind(&oppgave_row.melding_id)
        .bind(&oppgave_row.opplysninger)
        .bind(oppgave_row.arbeidssoeker_id)
        .bind(&oppgave_row.identitetsnummer)
        .bind(oppgave_row.tidspunkt)
        .fetch_one(&mut **transaction)
        .await?;

    Ok(oppgave_id)
}

pub async fn insert_oppgave_status_logg(
    status_logg_row: &OppgaveStatusLoggRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<u64, Box<dyn Error>> {
    let result = sqlx::query(
        r#"
        INSERT INTO oppgave_status_logg (oppgave_id, status, melding, tidspunkt)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(status_logg_row.oppgave_id)
    .bind(&status_logg_row.status)
    .bind(&status_logg_row.melding)
    .bind(status_logg_row.tidspunkt)
    .execute(&mut **transaction)
    .await?;

    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::oppgave_type::OppgaveType;
    use chrono::Utc;
    use paw_test::setup_test_db::setup_test_db;
    use std::error::Error;
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_oppgave_med_status_ubehandlet() -> Result<(), Box<dyn Error>> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let oppgave_row = OppgaveRow {
            type_: OppgaveType::AvvistUnder18.to_string(),
            melding_id: Uuid::new_v4(),
            opplysninger: vec![
                "ER_UNDER_18_AAR".to_string(),
                "BOSATT_ETTER_FREG_LOVEN".to_string()
            ],
            arbeidssoeker_id: 12345,
            identitetsnummer: "12345678901".to_string(),
            tidspunkt: Utc::now(),
        };

        let oppgave_id = insert_oppgave_med(OppgaveStatus::Ubehandlet, &oppgave_row, &mut tx).await?;
        tx.commit().await?;
        assert_eq!(oppgave_id, 1);

        Ok(())
    }
}
