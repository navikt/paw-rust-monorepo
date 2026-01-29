use sqlx::{Postgres, Transaction};
use std::error;

pub async fn update_hwm(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
    hwm: i64,
) -> Result<bool, Box<dyn error::Error>> {
    let res = sqlx::query(
        r#"
        update hwm
        set hwm = $4
        where version = $1 AND topic = $2 AND partition = $3 and hwm < $4;
        "#,
    )
        .bind(version)
        .bind(topic)
        .bind(partition)
        .bind(hwm)
        .execute(&mut **tx)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn insert_hwm(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
    hwm: i64,
) -> Result<(), Box<dyn error::Error>> {
    sqlx::query(
        r#"
        insert into hwm (version, topic, partition, hwm)
        values ($1, $2, $3, $4);
        "#,
    )
        .bind(version)
        .bind(topic)
        .bind(partition)
        .bind(hwm)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn get_hwm(
    tx: &mut Transaction<'_, Postgres>,
    version: i16,
    topic: &str,
    partition: i32,
) -> Result<Option<i64>, Box<dyn error::Error>> {
    let row = sqlx::query_scalar(
        r#"
        select hwm
        from hwm
        where version = $1 AND topic = $2 AND partition = $3;
        "#,
    )
        .bind(version)
        .bind(topic)
        .bind(partition)
        .fetch_optional(&mut **tx)
        .await?;

    Ok(row)
}