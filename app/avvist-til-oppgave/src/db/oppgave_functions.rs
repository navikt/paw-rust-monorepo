use crate::db::oppgave_row::{InsertOppgaveRow, OppgaveRow};
use crate::db::oppgave_status_logg_row::{InsertOppgaveStatusLoggRow, OppgaveStatusLoggRow};
use crate::domain::oppgave::Oppgave;
use crate::domain::status_logg_entry::StatusLoggEntry;
use sqlx::{Postgres, Transaction};
use std::error::Error;

pub async fn hent_oppgave(
    arbeidssoeker_id: i64,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Option<Oppgave>, sqlx::Error> {
    let oppgave_row = hent_oppgave_for_arbeidssoeker(arbeidssoeker_id, tx).await?;
    let oppgave_row = match oppgave_row {
        None => return Ok(None),
        Some(row) => row,
    };

    let status_logg: Vec<StatusLoggEntry> = hent_status_logg(oppgave_row.id, tx).await?;

    let oppgave = Oppgave::new(
        oppgave_row.id,
        oppgave_row.type_.parse().unwrap(),
        oppgave_row.status.parse().unwrap(),
        oppgave_row.opplysninger,
        oppgave_row.arbeidssoeker_id,
        oppgave_row.identitetsnummer,
        oppgave_row.tidspunkt,
        status_logg,
    );

    Ok(Some(oppgave))
}

async fn hent_oppgave_for_arbeidssoeker(
    arbeidssoeker_id: i64,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<OppgaveRow>, sqlx::Error> {
    sqlx::query_as::<_, OppgaveRow>(
        r#"
        SELECT
            id,
            type AS type_,
            status,
            melding_id,
            opplysninger,
            arbeidssoeker_id,
            identitetsnummer,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgaver
        WHERE arbeidssoeker_id = $1
        "#,
    )
    .bind(arbeidssoeker_id)
    .fetch_optional(&mut **transaction)
    .await
}

async fn hent_status_logg(
    oppgave_id: i64,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<StatusLoggEntry>, sqlx::Error> {
    let status_logg = sqlx::query_as::<_, OppgaveStatusLoggRow>(
        r#"
        SELECT
            id,
            oppgave_id,
            status,
            melding,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgave_status_logg
        WHERE oppgave_id = $1
        ORDER BY tidspunkt DESC
        "#,
    )
    .bind(oppgave_id)
    .fetch_all(&mut **transaction)
    .await?
    .into_iter()
    .map(|row| StatusLoggEntry::new(row.status, row.tidspunkt))
    .collect();
    Ok(status_logg)
}

pub async fn insert_oppgave_med(
    oppgave_row: &InsertOppgaveRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64, Box<dyn Error>> {
    let oppgave_id = insert_oppgave(oppgave_row, transaction).await?;

    let status_logg_row = InsertOppgaveStatusLoggRow {
        oppgave_id,
        status: oppgave_row.status.to_string(),
        melding: "Ubehandlet oppgave opprettet".to_string(),
        tidspunkt: oppgave_row.tidspunkt.clone(),
    };

    insert_oppgave_status_logg(&status_logg_row, transaction).await?;

    Ok(oppgave_id)
}

async fn insert_oppgave(
    oppgave_row: &InsertOppgaveRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64, Box<dyn Error>> {
    let oppgave_id = sqlx::query_scalar(
        r#"
        INSERT INTO oppgaver (type, status, melding_id, opplysninger, arbeidssoeker_id, identitetsnummer, tidspunkt)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id
        "#,
    )
        .bind(&oppgave_row.type_)
        .bind(&oppgave_row.status)
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
    status_logg_row: &InsertOppgaveStatusLoggRow,
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
    use crate::domain::oppgave_status::OppgaveStatus;
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

        let arbeidssoeker_id = 12345;
        let oppgave_row = test_oppgave_row(arbeidssoeker_id);

        {
            let oppgave_id = insert_oppgave_med(&oppgave_row, &mut tx).await?;
            tx.commit().await?;
            assert_eq!(oppgave_id, 1);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_hent_oppgave() -> Result<(), Box<dyn Error>> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Sett inn en oppgave
        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = 12345;
        let oppgave_row = test_oppgave_row(arbeidssoeker_id);

        insert_oppgave_med(&oppgave_row, &mut tx).await?;
        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let oppgave = hent_oppgave(arbeidssoeker_id, &mut tx).await?;
        tx.commit().await?;

        let oppgave = oppgave.unwrap();
        assert_eq!(oppgave.type_, OppgaveType::AvvistUnder18);
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);
        assert_eq!(oppgave.arbeidssoeker_id, 12345);
        assert_eq!(oppgave.identitetsnummer, "12345678901");
        assert_eq!(
            oppgave.opplysninger,
            vec!["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN"]
        );
        assert_eq!(oppgave.status_logg.len(), 1);
        assert_eq!(oppgave.status, OppgaveStatus::Ubehandlet);

        let mut tx = pg_pool.begin().await?;
        assert_eq!(hent_oppgave(99999, &mut tx).await?, None);

        Ok(())
    }

    fn test_oppgave_row(arbeidssoeker_id: i64) -> InsertOppgaveRow {
        InsertOppgaveRow {
            type_: OppgaveType::AvvistUnder18.to_string(),
            status: OppgaveStatus::Ubehandlet.to_string(),
            melding_id: Uuid::new_v4(),
            opplysninger: vec![
                "ER_UNDER_18_AAR".to_string(),
                "BOSATT_ETTER_FREG_LOVEN".to_string(),
            ],
            arbeidssoeker_id,
            identitetsnummer: "12345678901".to_string(),
            tidspunkt: Utc::now(),
        }
    }
}
