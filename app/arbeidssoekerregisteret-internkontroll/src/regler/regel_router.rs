use std::error::Error;

use sqlx::{Postgres, Transaction};

use crate::regler::regel_katalog::RegelId;

pub async fn exec(
    tx: Transaction<'_, Postgres>,
    regel_id: RegelId,
    xid: i64,
) -> Result<(), Box<dyn Error>> {
    match regel_id {
        RegelId::HeleTraceSignert => todo!("Implementer regel for HeleTraceSignert"),
        RegelId::IngenPeriodeUtenStartet => todo!("Implementer regel for IngenPeriodeUtenStartet"),
    }
}
