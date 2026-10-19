use sqlx::Acquire;
use sqlx::PgPool;
use tracing::info;

const RESET_SQL: &str = include_str!("../reset.sql");
pub async fn reset_db_if_needed(
    pool: &PgPool,
    configured_hwm_version: i16,
) -> Result<bool, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
        .fetch_one(&mut *connection)
        .await?;
    if !hwm_exists {
        info!("HWM-tabellen mangler, hopper over nullstilling av DB");
        return Ok(false);
    }
    let max_hwm_version: Option<i16> = sqlx::query_scalar("SELECT MAX(version) FROM public.hwm")
        .fetch_one(&mut *connection)
        .await?;
    match max_hwm_version {
        Some(version) if configured_hwm_version <= version => {
            info!(
                configured_hwm_version,
                stored_hwm_version = version,
                "Konfigurert HWM-versjon er ikke nyere enn lagret versjon, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(version) => {
            info!(
                configured_hwm_version,
                stored_hwm_version = version,
                "Konfigurert HWM-versjon er nyere enn lagret versjon, nullstiller DB"
            );
        }
        None => {
            info!(
                configured_hwm_version,
                "HWM-tabellen er tom, nullstiller DB"
            );
        }
    }
    let mut transaction = connection.begin().await?;
    sqlx::raw_sql(RESET_SQL).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(true)
}
