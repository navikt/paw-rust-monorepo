use crate::avvist_melding_status::AvvistMeldingStatus;
use crate::db::avvist_melding_row::AvvistMeldingRow;
use sqlx::{Postgres, Transaction};

pub async fn insert_ubehandlet_avvist_melding(
    avvist_melding_row: &AvvistMeldingRow,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO avvist_meldinger (melding_id, aarsak, identitetsnummer, arbeidssoeker_id, tidspunkt)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
        .bind(&avvist_melding_row.melding_id)
        .bind(&avvist_melding_row.aarsak)
        .bind(&avvist_melding_row.identitetsnummer)
        .bind(avvist_melding_row.arbeidssoeker_id)
        .bind(&avvist_melding_row.tidspunkt)
        .execute(&mut **transaction)
        .await?;
    println!("Resultat av første: {:?}", result.rows_affected());

    // Sett inn den første statusen
    let andre_result = sqlx::query(
        r#"
        INSERT INTO avvist_melding_status_logg (melding_id, status, tidspunkt)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(&avvist_melding_row.melding_id)
    .bind(AvvistMeldingStatus::Ubehandlet.to_str())
    .bind(avvist_melding_row.tidspunkt)
    .execute(&mut **transaction)
    .await?;
    println!("Resultat av andre: {:?}", andre_result.rows_affected());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::PgPool;
    use sqlx::postgres::PgPoolOptions;
    use std::error::Error;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{ContainerAsync, ImageExt};
    use testcontainers_modules::postgres::Postgres;
    use uuid::Uuid;

    #[tokio::test]
    #[ignore]
    async fn test_insert_ubehandlet_avvist_melding() {
        let (pg_pool, _container) = setup_test_db().await.unwrap();
        let første_januar_2023: f64 = 1672531200.0;
        let avvist_melding = AvvistMeldingRow {
            melding_id: Uuid::new_v4(),
            aarsak: "Er under 18 år".to_string(),
            identitetsnummer: "12345678901".to_string(),
            arbeidssoeker_id: 1,
            tidspunkt: første_januar_2023,
        };

        let mut tx = pg_pool.begin().await.unwrap();

        assert!(
            insert_ubehandlet_avvist_melding(&avvist_melding, &mut tx)
                .await
                .is_ok()
        );
    }

    async fn setup_test_db() -> Result<(PgPool, ContainerAsync<Postgres>), Box<dyn Error>> {
        let postgres_container = Postgres::default().with_tag("18-alpine").start().await?;

        let host_port = postgres_container.get_host_port_ipv4(5432).await?;
        let connection_string = format!(
            "postgresql://postgres:postgres@127.0.0.1:{}/postgres",
            host_port
        );

        unsafe {
            std::env::set_var("DATABASE_URL", &connection_string);
            std::env::set_var("PG_HOST", "127.0.0.1");
            std::env::set_var("PG_PORT", host_port.to_string());
            std::env::set_var("PG_USERNAME", "postgres");
            std::env::set_var("PG_PASSWORD", "postgres");
            std::env::set_var("PG_DATABASE_NAME", "postgres");
        }

        let pool = PgPoolOptions::new()
            .min_connections(1)
            .max_connections(3)
            .connect(&connection_string)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| -> Box<dyn Error> {
                println!("Migrering feilet: {}", e);
                format!("Migrering feilet: {}", e).into()
            })?;

        Ok((pool, postgres_container))
    }
}
