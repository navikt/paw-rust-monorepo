use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct KontortilknytningRow {
    pub id: Uuid,
    pub aktor_id: String,
    pub identitetsnummer: String,
    pub kontor_id: String,
    pub kontor_navn: String,
    pub kontor_type: String,
    pub tidspunkt: DateTime<Utc>,
}

impl KontortilknytningRow {
    pub fn new(
        id: Uuid,
        aktor_id: String,
        identitetsnummer: String,
        kontor_id: String,
        kontor_navn: String,
        kontor_type: String,
        tidspunkt: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt,
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn count_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<i64> {
    tracing::debug!("Count kontortilknytning by id");
    let count = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM kontortilknytninger
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(count)
}

#[allow(unused)]
#[tracing::instrument(skip_all)]
pub async fn select_by_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    id: &'a Uuid,
) -> anyhow::Result<Option<KontortilknytningRow>> {
    tracing::debug!("Select kontortilknytning by id");
    let rows = sqlx::query_as::<_, KontortilknytningRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt  AT TIME ZONE 'UTC' AS tidspunkt
        FROM kontortilknytninger
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_aktor_id<'a>(
    tx: &mut Transaction<'_, Postgres>,
    aktor_id: &'a str,
) -> anyhow::Result<Vec<KontortilknytningRow>> {
    tracing::debug!("Select kontortilknytning by aktor_id");
    let rows = sqlx::query_as::<_, KontortilknytningRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt  AT TIME ZONE 'UTC' AS tidspunkt
        FROM kontortilknytninger
        WHERE aktor_id = $1
        "#,
    )
    .bind(aktor_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_aktor_ids(
    tx: &mut Transaction<'_, Postgres>,
    aktor_ider: &[String],
) -> anyhow::Result<Vec<KontortilknytningRow>> {
    tracing::debug!("Select kontortilknytning by aktor_ider");
    let rows = sqlx::query_as::<_, KontortilknytningRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt  AT TIME ZONE 'UTC' AS tidspunkt
        FROM kontortilknytninger
        WHERE aktor_id = ANY($1)
        "#,
    )
    .bind(aktor_ider)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KontortilknytningRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert kontortilknytning");
    let result = sqlx::query(
        r#"
        INSERT INTO kontortilknytninger (
            id,
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
    .bind(&row.id)
    .bind(&row.aktor_id)
    .bind(&row.identitetsnummer)
    .bind(&row.kontor_id)
    .bind(&row.kontor_navn)
    .bind(&row.kontor_type)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all)]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a KontortilknytningRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update kontortilknytning");
    let result = sqlx::query(
        r#"
        UPDATE kontortilknytninger SET (
            aktor_id,
            identitetsnummer,
            kontor_id,
            kontor_navn,
            kontor_type,
            tidspunkt,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6, $7, $8) WHERE id = $1
        "#,
    )
    .bind(&row.id)
    .bind(&row.aktor_id)
    .bind(&row.identitetsnummer)
    .bind(&row.kontor_id)
    .bind(&row.kontor_navn)
    .bind(&row.kontor_type)
    .bind(&row.tidspunkt)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[tracing::instrument(skip_all)]
pub async fn delete<'a>(tx: &mut Transaction<'_, Postgres>, id: &'a Uuid) -> anyhow::Result<u64> {
    tracing::debug!("Delete kontortilknytning");
    let result = sqlx::query(
        r#"
        DELETE FROM kontortilknytninger WHERE id = $1
        "#,
    )
    .bind(&id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use tokio::sync::OnceCell;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn select_by_aktor_ids_grupperer_treff_per_aktor_id() {
        let pg_pool = init().await;
        let mut tx = pg_pool.begin().await.expect("Kunne ikke starte transaksjon");

        let aktor_id_1 = "aktor-batch-1";
        let aktor_id_2 = "aktor-batch-2";
        let naa = Utc::now();

        insert(
            &mut tx,
            &KontortilknytningRow::new(
                Uuid::new_v4(),
                aktor_id_1.to_string(),
                "12345678901".to_string(),
                "0301".to_string(),
                "Oslo".to_string(),
                "ARENA".to_string(),
                naa,
            ),
        )
        .await
        .expect("Kunne ikke sette inn kontortilknytning for testoppsett");
        insert(
            &mut tx,
            &KontortilknytningRow::new(
                Uuid::new_v4(),
                aktor_id_1.to_string(),
                "12345678901".to_string(),
                "0301".to_string(),
                "Oslo".to_string(),
                "AO".to_string(),
                naa,
            ),
        )
        .await
        .expect("Kunne ikke sette inn kontortilknytning for testoppsett");
        insert(
            &mut tx,
            &KontortilknytningRow::new(
                Uuid::new_v4(),
                aktor_id_2.to_string(),
                "10987654321".to_string(),
                "1101".to_string(),
                "Stavanger".to_string(),
                "GEOGRAFISK_TILKNYTNING".to_string(),
                naa,
            ),
        )
        .await
        .expect("Kunne ikke sette inn kontortilknytning for testoppsett");

        let rows = select_by_aktor_ids(
            &mut tx,
            &[aktor_id_1.to_string(), aktor_id_2.to_string()],
        )
        .await
        .expect("Kunne ikke hente kontortilknytninger");

        tx.commit().await.expect("Kunne ikke commit transaksjon");

        let for_aktor_1: Vec<_> = rows.iter().filter(|r| r.aktor_id == aktor_id_1).collect();
        let for_aktor_2: Vec<_> = rows.iter().filter(|r| r.aktor_id == aktor_id_2).collect();
        assert_eq!(for_aktor_1.len(), 2);
        assert_eq!(for_aktor_2.len(), 1);
    }

    static INIT: OnceCell<PgPool> = OnceCell::const_new();

    async fn init() -> &'static PgPool {
        INIT.get_or_init(|| async {
            let postgres_guard = setup_postgres_container()
                .await
                .expect("Failed to start Postgres container");
            sqlx::migrate!("./migrations")
                .run(&postgres_guard.pg_pool)
                .await
                .expect("Failed to run migrations");
            postgres_guard.pg_pool
        })
        .await
    }
}
