use std::num::NonZeroU16;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::instrument;

use crate::dao::perioder::{KontrollStatusType, oppdater_kontroll_status};
use regler_arbeidssoeker::regler::regelsett::Regelsett;

use super::evaluer::{KontrollStatus, sjekk_status};
use super::hent_data::hent_perioder_for_kontroll;

#[derive(Clone)]
pub struct KontrollTask {
    inner: Arc<KontrollTaskInner>,
}

struct KontrollTaskInner {
    pg_pool: PgPool,
    batch_size: NonZeroU16,
    regelsett: Regelsett,
}

impl KontrollTask {
    pub fn new(pg_pool: PgPool, batch_size: NonZeroU16, regelsett: Regelsett) -> Self {
        Self {
            inner: Arc::new(KontrollTaskInner {
                pg_pool,
                batch_size,
                regelsett,
            }),
        }
    }

    #[instrument(skip(self))]
    pub async fn kjoer_kontroll(&self) -> Result<bool> {
        let mut tx = self.inner.pg_pool.begin().await?;
        let perioder = hent_perioder_for_kontroll(&mut tx, self.inner.batch_size).await?;
        if perioder.is_empty() {
            return Ok(false);
        }
        tracing::info!("Kontrollerer {} perioder", perioder.len());

        for periode in &perioder {
            let Some(gjeldende) = &periode.gjeldende_opplysninger else {
                tracing::warn!(
                    periode_id = ?periode.id,
                    "Periode trenger kontroll men mangler gjeldende opplysninger"
                );
                continue;
            };

            let gjeldende_vec: Vec<_> = gjeldende.0.iter().cloned().collect();
            let gjeldende_resultat = self.inner.regelsett.evaluer(&gjeldende_vec);

            let forrige_resultat = periode.forrige_opplysninger.as_ref().map(|opl| {
                let vec: Vec<_> = opl.0.iter().cloned().collect();
                self.inner.regelsett.evaluer(&vec)
            });

            let status = sjekk_status(Some(gjeldende_resultat), forrige_resultat);

            let (db_status, opplysninger) = match &status {
                Ok(KontrollStatus::IngenEndring) => {
                    (KontrollStatusType::Godkjent, None)
                }
                Ok(KontrollStatus::Endret(Err(_))) => {
                    (KontrollStatusType::Avvist, Some(gjeldende))
                }
                Ok(KontrollStatus::Endret(Ok(_))) => {
                    (KontrollStatusType::Godkjent, Some(gjeldende))
                }
                Err(e) => {
                    tracing::warn!(
                        periode_id = ?periode.id,
                        "Kontroll feilet: {}",
                        e
                    );
                    continue;
                }
            };

            oppdater_kontroll_status(
                &mut tx,
                &periode.id,
                &db_status,
                Utc::now(),
                opplysninger,
            )
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }
}

pub fn start_kontroll_task(
    kontroll: KontrollTask,
    intervall: std::time::Duration,
) -> JoinHandle<Result<()>> {
    tokio::spawn(async move {
        loop {
            let hadde_arbeid = kontroll.kjoer_kontroll().await?;
            if !hadde_arbeid {
                sleep(intervall).await;
            }
        }
    })
}
