use chrono::{DateTime, Utc};
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub(crate) struct LedighetsperiodeV2Row {
    #[allow(unused)]
    pub arbeidssoeker_id: i64,
    pub periode_id: Uuid,
    pub arbeidssoeker_fra: DateTime<Utc>,
    pub arbeidsledig_fra: Option<DateTime<Utc>>,
    #[allow(unused)]
    pub arbeidssoeker_til: Option<DateTime<Utc>>,
    pub egenvurdert_til: Option<String>,
    pub bekreftelse_har_jobbet: Option<bool>,
    pub bekreftelse_vil_fortsette: Option<bool>,
    pub bekreftelse_ansvar: Vec<String>,
}

#[tracing::instrument(skip_all)]
pub async fn select_by_arbeidssoeker_id(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: i64,
) -> anyhow::Result<Vec<LedighetsperiodeV2Row>> {
    tracing::debug!("Select ledighetsperioder by parent-id");
    let rows = sqlx::query_as::<_, LedighetsperiodeV2Row>(
        r#"
        WITH
        latest_egenvurderinger AS (
            SELECT periode_id, egenvurdert_til
            FROM egenvurderinger
            ORDER BY tidspunkt DESC
            LIMIT 1
        ),
        latest_bekreftelser AS (
            SELECT periode_id, har_jobbet, vil_fortsette, bekreftelsesloesning
            FROM bekreftelser
            ORDER BY gjelder_til DESC
            LIMIT 1
        )
        SELECT
            k.arbeidssoeker_id,
            k.periode_id,
            k.arbeidssoeker_fra AT TIME ZONE 'UTC'                  AS arbeidssoeker_fra,
            k.arbeidssoeker_til AT TIME ZONE 'UTC'                  AS arbeidssoeker_til,
            k.arbeidsledig_fra AT TIME ZONE 'UTC'                   AS arbeidsledig_fra,
            e.egenvurdert_til,
            b.har_jobbet                                            AS bekreftelse_har_jobbet,
            b.vil_fortsette                                         AS bekreftelse_vil_fortsette,
            COALESCE(bv.bekreftelsesloesninger, ARRAY[]::varchar[]) AS bekreftelse_ansvar
        FROM kartlegginger k
        LEFT JOIN latest_egenvurderinger e    ON e.periode_id  = k.periode_id
        LEFT JOIN latest_bekreftelser b       ON b.periode_id  = k.periode_id
        LEFT JOIN bekreftelse_paavegneav bv   ON bv.periode_id = k.periode_id
        WHERE k.arbeidssoeker_id = $1 AND k.arbeidssoeker_til IS NOT NULL
        ORDER BY k.arbeidsledig_fra DESC NULLS LAST, k.arbeidssoeker_fra DESC
        LIMIT 1
        "#,
    )
    .bind(arbeidssoeker_id)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    #[tokio::test]
    async fn sorterer_nyeste_arbeidsledighet_forst_og_null_sist() -> anyhow::Result<()> {
        let postgres = setup_postgres_container().await?;
        sqlx::migrate!("./migrations")
            .run(&postgres.pg_pool)
            .await?;
        let mut tx = postgres.pg_pool.begin().await?;
        let arbeidssoeker_id = 545;

        sqlx::query(
            r#"
            INSERT INTO arbeidssoekere (
                id, aktor_id, identitetsnummer, inserted_timestamp
            ) VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(arbeidssoeker_id)
        .bind("aktor-id")
        .bind("12345678901")
        .bind(Utc::now())
        .execute(&mut *tx)
        .await?;

        let nyeste_ledighet = Uuid::new_v4();
        let eldre_ledighet = Uuid::new_v4();
        let nyeste_arbeidssoeker_uten_ledighet = Uuid::new_v4();
        let eldre_arbeidssoeker_uten_ledighet = Uuid::new_v4();
        let kartlegginger = [
            (
                eldre_ledighet,
                timestamp("2025-03-01T00:00:00Z"),
                Some(timestamp("2025-04-01T05:56:24.644Z")),
            ),
            (
                nyeste_ledighet,
                timestamp("2026-03-01T00:00:00Z"),
                Some(timestamp("2026-04-30T08:02:54.302Z")),
            ),
            (
                eldre_arbeidssoeker_uten_ledighet,
                timestamp("2026-05-01T00:00:00Z"),
                None,
            ),
            (
                nyeste_arbeidssoeker_uten_ledighet,
                timestamp("2027-05-01T00:00:00Z"),
                None,
            ),
        ];

        for (periode_id, arbeidssoeker_fra, arbeidsledig_fra) in kartlegginger {
            sqlx::query(
                r#"
                INSERT INTO kartlegginger (
                    periode_id,
                    arbeidssoeker_id,
                    arbeidssoeker_fra,
                    arbeidsledig_fra,
                    inserted_timestamp
                ) VALUES ($1, $2, $3, $4, $5)
                "#,
            )
            .bind(periode_id)
            .bind(arbeidssoeker_id)
            .bind(arbeidssoeker_fra)
            .bind(arbeidsledig_fra)
            .bind(Utc::now())
            .execute(&mut *tx)
            .await?;
        }

        let rows = select_by_arbeidssoeker_id(&mut tx, arbeidssoeker_id).await?;
        let periode_ids = rows.iter().map(|row| row.periode_id).collect::<Vec<_>>();
        assert_eq!(
            periode_ids,
            vec![
                nyeste_ledighet,
                eldre_ledighet,
                nyeste_arbeidssoeker_uten_ledighet,
                eldre_arbeidssoeker_uten_ledighet,
            ]
        );

        let paged_rows = select_by_arbeidssoeker_id(&mut tx, arbeidssoeker_id).await?;
        let paged_periode_ids = paged_rows
            .iter()
            .map(|row| row.periode_id)
            .collect::<Vec<_>>();
        assert_eq!(
            paged_periode_ids,
            vec![eldre_ledighet, nyeste_arbeidssoeker_uten_ledighet]
        );

        Ok(())
    }

    fn timestamp(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .expect("timestamp should be valid RFC 3339")
            .with_timezone(&Utc)
    }
}
