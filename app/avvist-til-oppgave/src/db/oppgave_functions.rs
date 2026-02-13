use crate::db::oppgave_hendelse_logg_row::{InsertOppgaveHendelseLoggRow, OppgaveHendelseLoggRow};
use crate::db::oppgave_row::{InsertOppgaveRow, OppgaveRow};
use crate::domain::hendelse_logg_entry::{HendelseLoggEntry, HendelseLoggEntryError};
use crate::domain::hendelse_logg_status::HendelseLoggStatus;
use crate::domain::oppgave::Oppgave;
use anyhow::Result;
use sqlx::{Postgres, Transaction};

pub async fn hent_oppgave(
    arbeidssoeker_id: i64,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Option<Oppgave>> {
    let oppgave_row = match hent_oppgave_for_arbeidssoeker(arbeidssoeker_id, tx).await? {
        None => return Ok(None),
        Some(row) => row,
    };

    let hendelse_logg: Vec<HendelseLoggEntry> = hent_hendelse_logg(oppgave_row.id, tx).await?;

    let oppgave = Oppgave::new(
        oppgave_row.id,
        oppgave_row.type_,
        oppgave_row.status,
        oppgave_row.opplysninger,
        oppgave_row.arbeidssoeker_id,
        oppgave_row.identitetsnummer,
        oppgave_row.tidspunkt,
        hendelse_logg,
    )?;

    Ok(Some(oppgave))
}

async fn hent_oppgave_for_arbeidssoeker(
    arbeidssoeker_id: i64,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<OppgaveRow>> {
    let rows = sqlx::query_as::<_, OppgaveRow>(
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
    .await?;
    Ok(rows)
}

async fn hent_hendelse_logg(
    oppgave_id: i64,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<HendelseLoggEntry>> {
    let rows = sqlx::query_as::<_, OppgaveHendelseLoggRow>(
        r#"
        SELECT
            id,
            oppgave_id,
            status,
            melding,
            tidspunkt AT TIME ZONE 'UTC' as tidspunkt
        FROM oppgave_hendelse_logg
        WHERE oppgave_id = $1
        ORDER BY tidspunkt DESC
        "#,
    )
    .bind(oppgave_id)
    .fetch_all(&mut **transaction)
    .await?;

    let hendelse_logg: Vec<HendelseLoggEntry> =
        rows.into_iter().try_fold(Vec::new(), |mut acc, row| {
            let entry = HendelseLoggEntry::new(row.status, row.tidspunkt)?;
            acc.push(entry);
            Ok::<Vec<HendelseLoggEntry>, HendelseLoggEntryError>(acc)
        })?;

    Ok(hendelse_logg)
}

pub async fn insert_oppgave(
    oppgave_row: &InsertOppgaveRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64> {
    let oppgave_id = _insert_oppgave(oppgave_row, transaction).await?;

    let hendelse_logg_row = InsertOppgaveHendelseLoggRow {
        oppgave_id,
        status: HendelseLoggStatus::OppgaveOpprettet.to_string(),
        melding: "Oppretter oppgave for avvist hendelse".to_string(),
        tidspunkt: oppgave_row.tidspunkt.clone(),
    };

    insert_oppgave_hendelse_logg(&hendelse_logg_row, transaction).await?;

    Ok(oppgave_id)
}

async fn _insert_oppgave(
    oppgave_row: &InsertOppgaveRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<i64> {
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

pub async fn insert_oppgave_hendelse_logg(
    hendelse_logg_row: &InsertOppgaveHendelseLoggRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<u64> {
    let result = sqlx::query(
        r#"
        INSERT INTO oppgave_hendelse_logg (oppgave_id, status, melding, tidspunkt)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(hendelse_logg_row.oppgave_id)
    .bind(&hendelse_logg_row.status)
    .bind(&hendelse_logg_row.melding)
    .bind(hendelse_logg_row.tidspunkt)
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
    use uuid::Uuid;

    #[tokio::test]
    async fn insert_oppgave_med_status_ubehandlet() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let arbeidssoeker_id = 12345;
        let oppgave_row = test_oppgave_row(arbeidssoeker_id);

        let oppgave_id = insert_oppgave(&oppgave_row, &mut tx).await?;
        tx.commit().await?;
        assert_eq!(oppgave_id, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_hent_oppgave() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Sett inn en oppgave
        let mut tx = pg_pool.begin().await?;
        let arbeidssoeker_id = 12345;
        let oppgave_row = test_oppgave_row(arbeidssoeker_id);

        insert_oppgave(&oppgave_row, &mut tx).await?;
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
        assert_eq!(oppgave.hendelse_logg.len(), 1);
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
