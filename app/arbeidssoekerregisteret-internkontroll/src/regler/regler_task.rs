use std::{num::NonZeroU32, sync::Arc};

use paw_observability::health::simple_app_state::AppState;
use tokio::time::Interval;
use tracing::Span;

use crate::{
    database::{
        funn::{delete_funn, upsert_funn},
        record_derived::{get_neste_record_derived_xids, get_record_derived_at_xid},
        regel_fremdrift::{RegelFremdrift, get_regel_fremdrift, upsert_regel_fremdrift},
    },
    regler::{
        funn::Funn::{AltOk, Feil, IkkeAktuell},
        nokkel::{Nivaa, Nokkel},
        regel_katalog::RegelId,
        regel_router::exec,
    },
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Regel {
    id: RegelId,
    versjon: i32,
    nivaa: Nivaa,
}

impl Regel {
    pub fn new(id: RegelId, versjon: i32, nivaa: Nivaa) -> Self {
        Self { id, versjon, nivaa }
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
        let mut antall_prosessert = 0_usize;
        loop {
            if !app_state.is_alive() {
                return Ok(());
            }
            if antall_prosessert == 0 {
                interval.tick().await;
            }
            match exec_regler(&pg_pool, &regler, batch_size).await {
                Ok(antall) => antall_prosessert = antall,
                Err(err) => {
                    app_state.set_is_alive(false);
                    return Err(err);
                }
            }
        }
    })
}

#[tracing::instrument(skip(pg_pool))]
pub async fn exec_regler(
    pg_pool: &sqlx::PgPool,
    regler: &[Regel],
    batch_size: NonZeroU32,
) -> Result<usize, sqlx::Error> {
    let mut tx = pg_pool.begin().await?;
    let mut fremdrift = get_regel_fremdrift(&mut tx).await?;
    for regel in regler {
        if !fremdrift
            .iter()
            .any(|rf| rf.regel == regel.id && rf.regelversjon == regel.versjon)
        {
            let lagt_til = upsert_regel_fremdrift(&mut tx, regel.id, regel.versjon, 0).await?;
            if lagt_til {
                fremdrift.push(RegelFremdrift {
                    regel: regel.id,
                    regelversjon: regel.versjon,
                    siste_xid: 0,
                });
            }
        }
    }
    let fremdrift = fremdrift;
    let Some(regel_fremdrift) = fremdrift
        .iter()
        .filter(|rf| {
            regler
                .iter()
                .any(|r| r.id == rf.regel && r.versjon == rf.regelversjon)
        })
        .min_by_key(|rf| rf.siste_xid)
        .cloned()
    else {
        Span::current().record("antall_nye_xids", 0);
        return Ok(0);
    };
    let regel_id = regel_fremdrift.regel;
    let regel_versjon = regel_fremdrift.regelversjon;
    let nivaa = regler
        .iter()
        .find(|r| r.id == regel_id && r.versjon == regel_versjon)
        .map(|r| r.nivaa)
        .expect("fremdrift er filtrert på registrerte regler");
    let neste_xids =
        get_neste_record_derived_xids(&mut tx, regel_fremdrift.siste_xid, batch_size).await?;
    let antall_nye_xids = neste_xids.len();
    for neste_xid in neste_xids {
        let klar = upsert_regel_fremdrift(&mut tx, regel_id, regel_versjon, neste_xid).await?;
        if !klar {
            break;
        }
        let Some((record, stream_ms)) = get_record_derived_at_xid(&mut tx, neste_xid).await? else {
            continue;
        };
        match exec(&mut tx, regel_id, neste_xid, &record).await? {
            IkkeAktuell => {}
            AltOk(nokkel) => {
                sjekk_nivaa(regel_id, nivaa, &nokkel)?;
                delete_funn(&mut tx, regel_id, regel_versjon, &nokkel).await?
            }
            Feil(nokkel, utfall) => {
                sjekk_nivaa(regel_id, nivaa, &nokkel)?;
                upsert_funn(
                    &mut tx,
                    regel_id,
                    regel_versjon,
                    &nokkel,
                    &utfall,
                    stream_ms,
                )
                .await?
            }
        }
    }
    tx.commit().await?;
    Span::current().record("antall_nye_xids", antall_nye_xids);
    Ok(antall_nye_xids)
}

/// A rule returning a key on another level than it declares is a
/// programming error: the finding would land in the wrong table.
fn sjekk_nivaa(regel_id: RegelId, nivaa: Nivaa, nokkel: &Nokkel) -> Result<(), sqlx::Error> {
    if nokkel.nivaa() == nivaa {
        Ok(())
    } else {
        Err(sqlx::Error::Protocol(format!(
            "regel {regel_id:?} er deklarert på nivå {} men returnerte nøkkel på nivå {}",
            nivaa.as_str(),
            nokkel.nivaa().as_str()
        )))
    }
}
