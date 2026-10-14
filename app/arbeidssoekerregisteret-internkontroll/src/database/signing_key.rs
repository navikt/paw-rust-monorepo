use chrono::{DateTime, Utc};
use sqlx::PgExecutor;

/// Revocation of a signing key, defined in code so that git tracks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revocation {
    /// The last stream time (ms) the key is accepted for.
    pub revoked_after_ms: i64,
    /// When the revocation was registered in code.
    /// Records derived before this time may need a new derivation.
    pub recorded_at: DateTime<Utc>,
}

/// A signing key as defined in code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SigningKeyEntry<'a> {
    pub key_id: &'a str,
    pub revocation: Option<Revocation>,
}

/// Writes the signing keys from code to the `signing_key` table.
///
/// New keys are inserted. Existing keys get the revocation from code;
/// rows that already match are not rewritten.
/// Keys missing from `keys` are left untouched: `record_derived` refers to them.
pub async fn sync_signing_keys(
    executor: impl PgExecutor<'_>,
    keys: &[SigningKeyEntry<'_>],
) -> Result<(), sqlx::Error> {
    let key_ids: Vec<&str> = keys.iter().map(|k| k.key_id).collect();
    let revoked_after_ms: Vec<Option<i64>> = keys
        .iter()
        .map(|k| k.revocation.map(|r| r.revoked_after_ms))
        .collect();
    let recorded_at: Vec<Option<DateTime<Utc>>> = keys
        .iter()
        .map(|k| k.revocation.map(|r| r.recorded_at))
        .collect();
    sqlx::query(
        r#"
        INSERT INTO signing_key (key_id, revoked_after_ms, revocation_recorded_at)
        SELECT key_id, revoked_after_ms, revocation_recorded_at
        FROM UNNEST($1::VARCHAR[], $2::BIGINT[], $3::TIMESTAMPTZ[])
            AS k (key_id, revoked_after_ms, revocation_recorded_at)
        ON CONFLICT (key_id) DO UPDATE
            SET revoked_after_ms = EXCLUDED.revoked_after_ms,
                revocation_recorded_at = EXCLUDED.revocation_recorded_at
            WHERE (signing_key.revoked_after_ms, signing_key.revocation_recorded_at)
                IS DISTINCT FROM (EXCLUDED.revoked_after_ms, EXCLUDED.revocation_recorded_at)
        "#,
    )
    .bind(key_ids)
    .bind(revoked_after_ms)
    .bind(recorded_at)
    .execute(executor)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    type Row = (String, Option<i64>, Option<DateTime<Utc>>);

    async fn rows(pool: &sqlx::PgPool) -> Vec<Row> {
        sqlx::query_as(
            "SELECT key_id, revoked_after_ms, revocation_recorded_at
             FROM signing_key ORDER BY key_id",
        )
        .fetch_all(pool)
        .await
        .unwrap()
    }

    fn revocation(revoked_after_ms: i64, recorded_at_ms: i64) -> Option<Revocation> {
        Some(Revocation {
            revoked_after_ms,
            recorded_at: DateTime::from_timestamp_millis(recorded_at_ms).unwrap(),
        })
    }

    #[tokio::test]
    async fn synkroniserer_nokler_med_tilbaketrekking_fra_kode() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = guard.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();

        let keys = [
            SigningKeyEntry { key_id: "a", revocation: None },
            SigningKeyEntry { key_id: "b", revocation: revocation(1_000, 5_000) },
        ];
        sync_signing_keys(&pool, &keys).await.unwrap();
        let first = rows(&pool).await;
        assert_eq!(first[0], ("a".to_string(), None, None));
        assert_eq!(
            first[1],
            ("b".to_string(), Some(1_000), DateTime::from_timestamp_millis(5_000))
        );

        // Restart without changes leaves the rows as they are.
        sync_signing_keys(&pool, &keys).await.unwrap();
        assert_eq!(rows(&pool).await, first);

        // Revoking "a" in code changes "a" only.
        let changed = [
            SigningKeyEntry { key_id: "a", revocation: revocation(2_000, 6_000) },
            keys[1],
        ];
        sync_signing_keys(&pool, &changed).await.unwrap();
        let after = rows(&pool).await;
        assert_eq!(
            after[0],
            ("a".to_string(), Some(2_000), DateTime::from_timestamp_millis(6_000))
        );
        assert_eq!(after[1], first[1]);
    }
}
