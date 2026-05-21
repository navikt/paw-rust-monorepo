use std::num::NonZeroU16;
use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use sqlx::PgPool;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::instrument;

use crate::dao::perioder::oppdater_trenger_kontroll;
use crate::dao::utgang_hendelse::{Input, InternUtgangHendelse};
use crate::dao::utgang_hendelser_logg::skriv_hendelser;
use crate::domain::utgang_hendelse_type::UtgangHendelseType;
use interne_hendelser::vo::BrukerType;
use regler_arbeidssoeker::regler::regelsett::Regelsett;

use super::evaluer::{KontrollStatus, sjekk_status};
use super::hent_data::{PeriodeKontrollData, hent_perioder_for_kontroll};

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

        let mut hendelser: Vec<InternUtgangHendelse<Input>> = Vec::new();
        let mut ferdig_kontrollert: Vec<types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId> =
            Vec::new();

        for periode in &perioder {
            let gjeldende_vec: Vec<_> =
                periode.gjeldende_opplysninger.0.iter().cloned().collect();
            let gjeldende_resultat = self.inner.regelsett.evaluer(&gjeldende_vec);

            let forrige_resultat = periode.forrige_opplysninger.as_ref().map(|opl| {
                let vec: Vec<_> = opl.0.iter().cloned().collect();
                self.inner.regelsett.evaluer(&vec)
            });

            let status = sjekk_status(Some(gjeldende_resultat), forrige_resultat);

            let hendelsetype = match &status {
                Ok(KontrollStatus::IngenEndring) => UtgangHendelseType::StatusIkkeEndret,
                Ok(KontrollStatus::Endret(Err(_))) => UtgangHendelseType::StatusEndretTilAvvist,
                Ok(KontrollStatus::Endret(Ok(_))) => UtgangHendelseType::StatusEndretTilOK,
                Err(e) => {
                    tracing::warn!(
                        periode_id = ?periode.periode_id,
                        "Kontroll feilet: {}",
                        e
                    );
                    continue;
                }
            };

            hendelser.push(InternUtgangHendelse::new(
                hendelsetype,
                periode.periode_id.clone(),
                Utc::now(),
                BrukerType::System,
                None,
            ));
            ferdig_kontrollert.push(periode.periode_id.clone());
        }

        skriv_hendelser(&mut tx, &hendelser).await?;
        oppdater_trenger_kontroll(&mut tx, &ferdig_kontrollert, false).await?;
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
