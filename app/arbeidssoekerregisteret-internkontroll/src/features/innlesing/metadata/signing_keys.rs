use std::{collections::HashMap, sync::Arc};

use sqlx::PgConnection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(transparent)]
pub struct SigningKeyDbId(i16);

#[derive(Clone, Debug)]
pub struct SigningKeyDbIds {
    map: Arc<HashMap<String, SigningKeyDbId>>,
}

impl SigningKeyDbIds {
    pub fn new(pairs: impl IntoIterator<Item = (impl Into<String>, SigningKeyDbId)>) -> Self {
        let map = pairs
            .into_iter()
            .map(|(key_id, db_id)| (key_id.into(), db_id))
            .collect();
        Self { map: Arc::new(map) }
    }

    pub async fn new_from_db(
        key_ids: impl IntoIterator<Item = &'static str>,
        conn: &mut PgConnection,
    ) -> Result<Self, sqlx::Error> {
        let mut map = HashMap::new();
        for key_id in key_ids {
            let db_id = match get_db_id(conn, key_id).await? {
                Some(id) => id,
                None => generate_db_id(conn, key_id).await?,
            };
            map.insert(key_id.to_string(), SigningKeyDbId(db_id));
        }
        Ok(Self { map: Arc::new(map) })
    }

    pub fn get(&self, key_id: &str) -> Option<SigningKeyDbId> {
        self.map.get(key_id).copied()
    }
}

pub(crate) async fn get_db_id(
    conn: &mut PgConnection,
    key_id: &str,
) -> Result<Option<i16>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT id FROM signing_key WHERE key_id = $1
        "#,
    )
    .bind(key_id)
    .fetch_one(&mut *conn)
    .await
}

pub(crate) async fn generate_db_id(
    conn: &mut PgConnection,
    key_id: &str,
) -> Result<i16, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        INSERT INTO signing_key (key_id)
        VALUES ($1)
        RETURNING id
        "#,
    )
    .bind(key_id)
    .fetch_one(&mut *conn)
    .await
}
