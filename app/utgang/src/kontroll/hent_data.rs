use std::num::NonZeroU16;

use tracing::instrument;

use crate::dao::perioder::{PeriodeRad, hent_perioder_som_trenger_kontroll};

#[instrument(skip(tx))]
pub async fn hent_perioder_for_kontroll(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    limit: NonZeroU16,
) -> Result<Vec<PeriodeRad>, sqlx::Error> {
    hent_perioder_som_trenger_kontroll(tx, limit).await
}
