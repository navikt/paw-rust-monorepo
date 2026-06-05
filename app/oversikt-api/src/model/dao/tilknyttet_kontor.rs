use sqlx::{FromRow, Postgres, Transaction};

#[derive(Debug, FromRow)]
pub(crate) struct TilknyttetKontorRow {
    pub id: i64,
    pub parent_id: i64,
    pub kontor_id: String,
    pub kontor_navn: String,
    pub kontor_type: String,
}

pub async fn select_by_parent_id(
    tx: &mut Transaction<'_, Postgres>,
    parent_id: &i64,
) -> anyhow::Result<Vec<TilknyttetKontorRow>> {
    let rows = sqlx::query_as::<_, TilknyttetKontorRow>(
        r#"
        SELECT id, parent_id, kontor_id, kontor_navn, kontor_type
        FROM tilknyttet_kontor
        WHERE parent_id = $1
        "#,
    )
    .bind(parent_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

pub async fn insert_rows(
    tx: &mut Transaction<'_, Postgres>,
    rows: &Vec<TilknyttetKontorRow>,
) -> anyhow::Result<Vec<i64>> {
    let mut ids = Vec::new();
    for row in rows {
        let id: i64 = insert_row(tx, row).await?;
        ids.push(id);
    }
    let ids = ids;
    Ok(ids)
}

pub async fn insert_row(
    tx: &mut Transaction<'_, Postgres>,
    row: &TilknyttetKontorRow,
) -> anyhow::Result<i64> {
    let id = sqlx::query_scalar(
        r#"
        INSERT INTO tilknyttet_kontor (parent_id, kontor_id, kontor_navn, kontor_type)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(row.parent_id)
    .bind(&row.kontor_id)
    .bind(&row.kontor_navn)
    .bind(&row.kontor_type)
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}
