use crate::model::sort::SortOrder;
use chrono::{NaiveDate, Utc};
use sqlx::{FromRow, Postgres, Transaction};

#[derive(Debug, FromRow)]
pub(crate) struct ArbeidssoekerRow {
    pub id: i64,
    pub aktor_id: String,
    pub identitetsnummer: String,
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
}

impl ArbeidssoekerRow {
    pub fn new(
        id: i64,
        aktor_id: String,
        identitetsnummer: String,
        fornavn: Option<String>,
        mellomnavn: Option<String>,
        etternavn: Option<String>,
    ) -> Self {
        Self {
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
        }
    }
}

#[tracing::instrument(skip_all)]
pub async fn count_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &Option<NaiveDate>,
) -> anyhow::Result<i64> {
    tracing::debug!("Count arbeidssøkere by kontortilknytning");
    let count = match ledig_siden {
        Some(timestamp) => {
            sqlx::query_scalar(
                r#"
            SELECT COUNT(DISTINCT a.id) AS count
            FROM arbeidssoekere a
            JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidssoeker_til IS NULL AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
            "#,
            ).bind(kontor_id)
                    .bind(&kontor_typer[..])
                    .bind(timestamp)
                    .fetch_one(&mut **tx)
                    .await?
        }
        None => {
            sqlx::query_scalar(
                r#"
            SELECT COUNT(DISTINCT a.id) AS count
            FROM arbeidssoekere a
            JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidssoeker_til IS NULL
            "#,
            ).bind(kontor_id)
                    .bind(&kontor_typer[..])
                    .fetch_one(&mut **tx)
                    .await?
        }
    };
    Ok(count)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_arbeidssoeker_id(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: &i64,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    tracing::debug!("Select arbeidssøkere by arbeidssoeker_id");
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM arbeidssoekere
        WHERE id = $1
        "#,
    )
    .bind(arbeidssoeker_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_identitetsnummer(
    tx: &mut Transaction<'_, Postgres>,
    identitetsnummer: &str,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    tracing::debug!("Select arbeidssøkere by identitetsnummer");
    let rows = sqlx::query_as::<_, ArbeidssoekerRow>(
        r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM arbeidssoekere
        WHERE identitetsnummer = $1
        "#,
    )
    .bind(identitetsnummer)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn select_by_kontortilknytning(
    tx: &mut Transaction<'_, Postgres>,
    kontor_id: &str,
    kontor_typer: &Vec<String>,
    ledig_siden: &Option<NaiveDate>,
    offset: i32,
    limit: i32,
    sort_order: &SortOrder,
) -> anyhow::Result<Vec<ArbeidssoekerRow>> {
    tracing::debug!("Select arbeidssøkere by kontortilknytning");
    let dir = sort_order.as_ref();
    let rows = match ledig_siden {
        None => {
            let sql = format!(
                // language=SQL
                r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM (
            SELECT DISTINCT ON (a.id)
                a.id,
                a.aktor_id,
                a.identitetsnummer,
                a.fornavn,
                a.mellomnavn,
                a.etternavn,
                k.arbeidsledig_fra AS sort_arbeidsledig_fra,
                k.arbeidssoeker_fra AS sort_arbeidssoeker_fra
            FROM arbeidssoekere a
            JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidssoeker_til IS NULL
            ORDER BY a.id, k.arbeidsledig_fra DESC NULLS LAST, k.arbeidssoeker_fra DESC
        ) distinct_arbeidssoekere
        ORDER BY sort_arbeidsledig_fra, sort_arbeidssoeker_fra {dir}
        OFFSET $3
        LIMIT $4
        "#,
            );
            sqlx::query_as::<_, ArbeidssoekerRow>(sqlx::AssertSqlSafe(sql))
                .bind(kontor_id)
                .bind(kontor_typer)
                .bind(offset)
                .bind(limit)
                .fetch_all(&mut **tx)
                .await?
        }
        Some(timestamp) => {
            let sql = format!(
                // language=SQL
                r#"
        SELECT
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn
        FROM (
            SELECT DISTINCT ON (a.id)
                a.id,
                a.aktor_id,
                a.identitetsnummer,
                a.fornavn,
                a.mellomnavn,
                a.etternavn,
                k.arbeidsledig_fra AS sort_arbeidsledig_fra,
                k.arbeidssoeker_fra AS sort_arbeidssoeker_fra
            FROM arbeidssoekere a
            JOIN kartlegginger k on a.id = k.arbeidssoeker_id
            JOIN kontortilknytninger kt on a.aktor_id = kt.aktor_id
            WHERE kt.kontor_id = $1 AND kt.kontor_type = ANY($2) AND k.arbeidssoeker_til IS NULL AND k.arbeidsledig_fra NOTNULL AND k.arbeidsledig_fra > $3
            ORDER BY a.id, k.arbeidsledig_fra DESC NULLS LAST, k.arbeidssoeker_fra DESC
        ) distinct_arbeidssoekere
        ORDER BY sort_arbeidsledig_fra, sort_arbeidssoeker_fra {dir}
        OFFSET $4
        LIMIT $5
        "#,
            );
            sqlx::query_as::<_, ArbeidssoekerRow>(sqlx::AssertSqlSafe(sql))
                .bind(kontor_id)
                .bind(kontor_typer)
                .bind(timestamp)
                .bind(offset)
                .bind(limit)
                .fetch_all(&mut **tx)
                .await?
        }
    };
    Ok(rows)
}

#[tracing::instrument(skip_all)]
pub async fn insert<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a ArbeidssoekerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Insert arbeidssøker");
    let result = sqlx::query(
        r#"
        INSERT INTO arbeidssoekere (
            id,
            aktor_id,
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            inserted_timestamp
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(&row.id)
    .bind(&row.aktor_id)
    .bind(&row.identitetsnummer)
    .bind(&row.fornavn)
    .bind(&row.mellomnavn)
    .bind(&row.etternavn)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[allow(unused)]
#[tracing::instrument(skip_all)]
pub async fn update<'a>(
    tx: &mut Transaction<'_, Postgres>,
    row: &'a ArbeidssoekerRow,
) -> anyhow::Result<u64> {
    tracing::debug!("Update arbeidssøker");
    let result = sqlx::query(
        r#"
        UPDATE arbeidssoekere SET (
            identitetsnummer,
            fornavn,
            mellomnavn,
            etternavn,
            updated_timestamp
        ) = ($2, $3, $4, $5, $6) WHERE id = $1
        "#,
    )
    .bind(row.id)
    .bind(&row.identitetsnummer)
    .bind(&row.fornavn)
    .bind(&row.mellomnavn)
    .bind(&row.etternavn)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dao::kartlegging;
    use crate::model::dao::kartlegging::KartleggingRow;
    use crate::model::dao::kontortilknytning;
    use crate::model::dao::kontortilknytning::KontortilknytningRow;
    use crate::model::dto::kontortilknytning::KontorType;
    use chrono::{DateTime, Duration};
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use uuid::Uuid;

    const NOE_KONTOR_ID: &str = "4242";

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn oppslag_returnerer_kun_arbeidssoekere_med_aktiv_kartlegging() {
        let context = TestContext::init().await;

        let mut tx = context.start_tx().await;

        let id_1: i64 = 20001; // Aktiv kartlegging med ledighet, én avsluttet med samme ledighet
        let id_2: i64 = 20002; // Aktiv kartlegging med ledighet, én avsluttet uten ledighet
        let id_3: i64 = 20003; // Aktiv kartlegging med ledighet, ingen avsluttet
        let id_4: i64 = 20004; // Aktiv kartlegging uten ledighet, én avsluttet med ledighet
        let id_5: i64 = 20005; // Avsluttet kartlegging med ledighet
        let id_6: i64 = 20006; // Avsluttet kartlegging uten ledighet
        let now = Utc::now();

        for id in [id_1, id_2, id_3, id_4, id_5, id_6] {
            context.insert_arbeidssoeker(&mut tx, id).await;
            context
                .insert_kontortilknytning(&mut tx, id, &KontorType::Arbeidsoppfolging)
                .await;
        }
        context
            .insert_kontortilknytning(&mut tx, id_1, &KontorType::Arena)
            .await;

        // Ingen kartlegginger
        let count = context.count_arbeidssoekere(&mut tx, &None).await;
        let rows = context.select_arbeidssoekere(&mut tx, &None).await;
        assert_eq!(count, 0);
        assert_eq!(rows.len(), 0);

        context
            .insert_avsluttet_kartlegging(
                &mut tx,
                id_1,
                now - Duration::days(21),
                now - Duration::days(11),
                Some(now - Duration::days(21)),
            )
            .await;
        context
            .insert_avsluttet_kartlegging(
                &mut tx,
                id_2,
                now - Duration::days(22),
                now - Duration::days(12),
                None,
            )
            .await;
        context
            .insert_avsluttet_kartlegging(
                &mut tx,
                id_4,
                now - Duration::days(24),
                now - Duration::days(14),
                Some(now - Duration::days(24)),
            )
            .await;
        context
            .insert_avsluttet_kartlegging(
                &mut tx,
                id_5,
                now - Duration::days(25),
                now - Duration::days(15),
                Some(now - Duration::days(25)),
            )
            .await;
        context
            .insert_avsluttet_kartlegging(
                &mut tx,
                id_6,
                now - Duration::days(26),
                now - Duration::days(16),
                None,
            )
            .await;

        // Kun avsluttede kartlegginger
        let count = context.count_arbeidssoekere(&mut tx, &None).await;
        let rows = context.select_arbeidssoekere(&mut tx, &None).await;
        assert_eq!(count, 0);
        assert_eq!(rows.len(), 0);

        context
            .insert_aktiv_kartlegging(
                &mut tx,
                id_1,
                now - Duration::days(10),
                Some(now - Duration::days(21)),
            )
            .await;
        context
            .insert_aktiv_kartlegging(
                &mut tx,
                id_2,
                now - Duration::days(11),
                Some(now - Duration::days(11)),
            )
            .await;
        context
            .insert_aktiv_kartlegging(
                &mut tx,
                id_3,
                now - Duration::days(12),
                Some(now - Duration::days(12)),
            )
            .await;
        context
            .insert_aktiv_kartlegging(&mut tx, id_4, now - Duration::days(13), None)
            .await;

        // Noen aktive kartlegginger
        let count = context.count_arbeidssoekere(&mut tx, &None).await;
        let rows = context.select_arbeidssoekere(&mut tx, &None).await;
        assert_eq!(count, 4);
        assert_eq!(rows.len(), 4);
        assert_eq!(
            rows.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![id_1, id_3, id_2, id_4]
        );

        tx.rollback()
            .await
            .expect("Kunne ikke rulle tilbake transaksjon");
    }

    struct TestContext {
        pg_pool: PgPool,
    }

    impl TestContext {
        async fn init() -> TestContext {
            {
                let postgres_guard = setup_postgres_container()
                    .await
                    .expect("Failed to start Postgres container");
                sqlx::migrate!("./migrations")
                    .run(&postgres_guard.pg_pool)
                    .await
                    .expect("Failed to run migrations");

                TestContext {
                    pg_pool: postgres_guard.pg_pool,
                }
            }
        }

        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }

        fn identitetsnummer(&self, id: i64) -> String {
            format!("010170{id}")
        }

        fn aktor_id(&self, id: i64) -> String {
            format!("2000{id}")
        }

        async fn count_arbeidssoekere(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            ledig_siden: &Option<DateTime<Utc>>,
        ) -> i64 {
            let kontor_typer: Vec<String> = vec![
                KontorType::Arena.as_ref().to_string(),
                KontorType::Arbeidsoppfolging.as_ref().to_string(),
                KontorType::GeografiskTilknytning.as_ref().to_string(),
            ];
            count_by_kontortilknytning(
                tx,
                NOE_KONTOR_ID,
                &kontor_typer,
                &ledig_siden.map(|d| d.date_naive()),
            )
            .await
            .expect("Kunne ikke telle arbeidssøkere")
        }

        async fn select_arbeidssoekere(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            ledig_siden: &Option<DateTime<Utc>>,
        ) -> Vec<ArbeidssoekerRow> {
            let kontor_typer: Vec<String> = vec![
                KontorType::Arena.as_ref().to_string(),
                KontorType::Arbeidsoppfolging.as_ref().to_string(),
                KontorType::GeografiskTilknytning.as_ref().to_string(),
            ];
            select_by_kontortilknytning(
                tx,
                NOE_KONTOR_ID,
                &kontor_typer,
                &ledig_siden.map(|d| d.date_naive()),
                0,
                100,
                &SortOrder::Descending,
            )
            .await
            .expect("Kunne ikke hente arbeidssøkere")
        }

        async fn insert_arbeidssoeker(&self, tx: &mut Transaction<'_, Postgres>, id: i64) {
            insert(
                tx,
                &ArbeidssoekerRow::new(
                    id,
                    self.aktor_id(id),
                    self.identitetsnummer(id),
                    None,
                    None,
                    None,
                ),
            )
            .await
            .expect("Kunne ikke sette inn arbeidssøker for testoppsett");
        }

        async fn insert_kontortilknytning(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            id: i64,
            kontor_type: &KontorType,
        ) {
            kontortilknytning::insert(
                tx,
                &KontortilknytningRow::new(
                    Uuid::new_v4(),
                    self.aktor_id(id),
                    self.identitetsnummer(id),
                    NOE_KONTOR_ID.to_string(),
                    "Testkontor".to_string(),
                    kontor_type.as_ref().to_string(),
                    Utc::now(),
                ),
            )
            .await
            .expect("Kunne ikke sette inn kontortilknytning for testoppsett");
        }

        async fn insert_aktiv_kartlegging(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            arbeidssoeker_id: i64,
            arbeidssoeker_fra: DateTime<Utc>,
            arbeidsledig_fra: Option<DateTime<Utc>>,
        ) {
            kartlegging::insert(
                tx,
                &KartleggingRow::new(
                    Uuid::new_v4(),
                    arbeidssoeker_id,
                    arbeidssoeker_fra,
                    None,
                    arbeidsledig_fra,
                ),
            )
            .await
            .expect("Kunne ikke sette inn kartlegging for testoppsett");
        }

        async fn insert_avsluttet_kartlegging(
            &self,
            tx: &mut Transaction<'_, Postgres>,
            arbeidssoeker_id: i64,
            arbeidssoeker_fra: DateTime<Utc>,
            arbeidssoeker_til: DateTime<Utc>,
            arbeidsledig_fra: Option<DateTime<Utc>>,
        ) {
            kartlegging::insert(
                tx,
                &KartleggingRow::new(
                    Uuid::new_v4(),
                    arbeidssoeker_id,
                    arbeidssoeker_fra,
                    Some(arbeidssoeker_til),
                    arbeidsledig_fra,
                ),
            )
            .await
            .expect("Kunne ikke sette inn kartlegging for testoppsett");
        }
    }
}
