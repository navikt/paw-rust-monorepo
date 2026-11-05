use sqlx::Acquire;
use sqlx::PgPool;
use tracing::info;

const RESET_SQL: &str = include_str!("../reset.sql");
#[derive(Debug, thiserror::Error)]
pub enum ResetDbError {
    #[error("stored HWM version {stored} is newer than configured version {configured}")]
    StoredVersionNewer { stored: i16, configured: i16 },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

pub async fn reset_db_if_needed(
    pool: &PgPool,
    configured_hwm_version: i16,
) -> Result<bool, ResetDbError> {
    let mut connection = pool.acquire().await?;
    let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('hwm') IS NOT NULL")
        .fetch_one(&mut *connection)
        .await?;
    if !hwm_exists {
        info!("HWM-tabellen mangler, hopper over nullstilling av DB");
        return Ok(false);
    }
    let max_hwm_version: Option<i16> = sqlx::query_scalar("SELECT MAX(version) FROM hwm")
        .fetch_one(&mut *connection)
        .await?;
    match max_hwm_version {
        None => {
            info!(
                configured_hwm_version,
                "HWM-tabellen er tom, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(stored) if stored == configured_hwm_version => {
            info!(
                configured_hwm_version,
                "HWM-versjonen er uendret, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(stored) if stored > configured_hwm_version => {
            return Err(ResetDbError::StoredVersionNewer {
                stored,
                configured: configured_hwm_version,
            });
        }
        Some(stored) => {
            info!(
                configured_hwm_version,
                stored_hwm_version = stored,
                "HWM-versjonen er økt, nullstiller DB"
            );
        }
    }
    let mut transaction = connection.begin().await?;
    sqlx::raw_sql(RESET_SQL).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(true)
}
