use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct BekreftelsePaaVegneAvRow {
    pub periode_id: Uuid,
    pub bekreftelsesloesninger: Vec<String>,
}

impl BekreftelsePaaVegneAvRow {
    pub fn new(periode_id: Uuid, bekreftelsesloesninger: Vec<String>) -> Self {
        Self {
            periode_id,
            bekreftelsesloesninger,
        }
    }
}

#[tracing::instrument(skip(tx, periode_id))]
pub async fn select_by_periode_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    periode_id: &'a Uuid,
) -> anyhow::Result<Vec<BekreftelsePaaVegneAvRow>> {
    tracing::debug!("Select bekreftelse_paavegneav by periode_id");
    let rows = sqlx::query_as::<_, BekreftelsePaaVegneAvRow>(
        r#"
        SELECT
            periode_id,
            bekreftelsesloesninger
        FROM bekreftelse_paavegneav
        WHERE periode_id = $1
        "#,
    )
    .bind(periode_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip(tx, row))]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a BekreftelsePaaVegneAvRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert bekreftelse_paavegneav");
    let result = sqlx::query(
        r#"
        INSERT INTO bekreftelse_paavegneav (
            periode_id,
            bekreftelsesloesninger
        ) VALUES ($1, $2)
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.bekreftelsesloesninger)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip(tx, row))]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a BekreftelsePaaVegneAvRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update bekreftelse_paavegneav");
    let result = sqlx::query(
        r#"
        UPDATE bekreftelse_paavegneav SET (
            bekreftelsesloesninger
        ) = ($2, $3, $4, $5, $6, $7, $8) WHERE periode_id = $1
        "#,
    )
    .bind(&row.periode_id)
    .bind(&row.bekreftelsesloesninger)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}
