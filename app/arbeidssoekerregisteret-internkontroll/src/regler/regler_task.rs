use std::{
    num::{NonZeroU16, NonZeroU32},
    sync::Arc,
};

use paw_observability::health::simple_app_state::AppState;
use tokio::time::Interval;

use crate::{
    database::{
        funn::{delete_funn, upsert_funn},
        record_derived::get_neste_record_derived_xids,
        regel_fremdrift::{get_regel_fremdrift, upsert_regel_fremdrift},
    },
    regler::{funn::Funn::IngenTraceId, regel_katalog::RegelId, regel_router::exec},
};

use super::funn::Funn::{AltOk, Feil};

pub struct Regel {
    id: RegelId,
    versjon: i32,
}

impl Regel {
    pub fn new(id: RegelId, versjon: i32) -> Self {
        Self { id, versjon }
    }
}

pub fn spawn_regel_task(
    pg_pool: sqlx::PgPool,
    app_state: Arc<AppState>,
    interval: Interval,
    regler: Vec<Regel>,
    batch_size: NonZeroU32,
) -> tokio::task::JoinHandle<Result<(), sqlx::Error>> {
    tokio::spawn(async move {
        let mut interval = interval;
        loop {
            if !app_state.is_alive() {
                return Ok(());
            }
            interval.tick().await;
            let res = exec_regler(&pg_pool, &regler, batch_size).await;
            if res.is_err() {
                app_state.set_is_alive(false);
                res?
            }
        }
    })
}

pub async fn exec_regler(
    pg_pool: &sqlx::PgPool,
    regler: &[Regel],
    batch_size: NonZeroU32,
) -> Result<(), sqlx::Error> {
    let mut tx = pg_pool.begin().await?;
    let Some(regel_fremdrift) = get_regel_fremdrift(&mut *tx)
        .await?
        .iter()
        .filter(|rf| {
            regler
                .iter()
                .any(|r| r.id == rf.regel && r.versjon == rf.regelversjon)
        })
        .min_by_key(|rf| rf.siste_xid)
        .cloned()
    else {
        return Ok(());
    };
    let regel_id = regel_fremdrift.regel;
    let regel_versjon = regel_fremdrift.regelversjon;
    let neste_xids =
        get_neste_record_derived_xids(&mut tx, regel_fremdrift.siste_xid, batch_size).await?;
    for neste_xid in neste_xids {
        let klar = upsert_regel_fremdrift(&mut tx, regel_id, regel_versjon, neste_xid).await?;
        if klar {
            let funn = exec(&mut tx, regel_id, neste_xid).await?;
            match funn {
                IngenTraceId => tracing::warn!("Ingen trace_id funner for xid {}", neste_xid),
                AltOk {
                    regel_id,
                    regel_versjon,
                    trace_id,
                } => delete_funn(&mut tx, trace_id, regel_id, regel_versjon).await?,
                Feil(detaljer) => {
                    upsert_funn(
                        &mut tx,
                        detaljer.trace_id,
                        detaljer.regel_id,
                        detaljer.regel_versjon,
                        detaljer.utfall,
                    )
                    .await?
                }
            }
        } else {
            break;
        }
    }
    tx.commit().await?;
    Ok(())
}
