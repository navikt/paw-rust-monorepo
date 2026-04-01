use std::{collections::{HashMap, HashSet}, num::NonZeroU16, sync::Arc};

use crate::{
    db_read_ops::hent_sist_oppdatert_foer_med_metadata,
    db_write_ops::{skriv_pdl_info_batch, skriv_status_batch},
    pdl::pdl_query::PDLClient,
    vo::status::Status,
};
use anyhow::Result;
use chrono::Duration;
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::fakta::{UtledeFakta, person_fakta::UtledePersonFakta};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

#[derive(Clone)]
pub struct PdlDataOppdatering {
    inner: Arc<PdlDataOppdateringRef>,
}

struct PdlDataOppdateringRef {
    pg_pool: PgPool,
    pdl_client: PDLClient,
    batch_size: NonZeroU16,
    intervall: Duration,
}

impl PdlDataOppdatering {
    pub fn new(
        pg_pool: PgPool,
        pdl_client: PDLClient,
        batch_size: NonZeroU16,
        intervall: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(PdlDataOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
                intervall,
            }),
        }
    }
    #[instrument(skip(self))]
    pub async fn kjoer_oppdatering(&self) -> Result<()> {
        tracing::info!("Starter oppdatering av PDL data");
        let pg_pool = &self.inner.pg_pool;
        let pdl_client = &self.inner.pdl_client;
        let batch_size = &self.inner.batch_size;

        let mut tx = pg_pool.begin().await?;
        let sist_oppdatert_foer = chrono::Utc::now() - self.inner.intervall;
        let skal_oppdateres = hent_sist_oppdatert_foer_med_metadata(
            &mut tx,
            &sist_oppdatert_foer,
            &[Status::Ok, Status::Avvist],
            batch_size,
        )
        .await?;
        let laast = skriv_status_batch(
            &mut tx,
            skal_oppdateres
                .iter()
                .map(|e| e.id)
                .collect::<Vec<Uuid>>()
                .as_slice(),
            &Status::Oppdateres,
            &chrono::Utc::now(),
        )
        .await?;
        let laast_set: HashSet<Uuid> = laast.into_iter().collect();
        let skal_oppdateres: Vec<_> = skal_oppdateres
            .into_iter()
            .filter(|e| laast_set.contains(&e.id))
            .collect();

        tracing::info!("{} perioder skal oppdateres", skal_oppdateres.len());
        if skal_oppdateres.is_empty() {
            return Ok(());
        }

        let identitetsnummer: Vec<String> = skal_oppdateres
            .iter()
            .map(|r| r.identitetsnummer.clone())
            .collect();
        let pdl_data = pdl_client
            .perform_hent_person_bolk(identitetsnummer)
            .await?;

        let utlede_person_fakta = UtledePersonFakta::default();
        let ident_til_person: HashMap<String, Result<Vec<Opplysning>>> = pdl_data
            .into_iter()
            .filter_map(|e| {
                e.person
                    .map(|p| (e.ident, utlede_person_fakta.utlede_fakta(&p)))
            })
            .collect();

        let batch: Vec<(Uuid, Vec<Opplysning>)> = skal_oppdateres
            .iter()
            .filter_map(
                |periode| match ident_til_person.get(&periode.identitetsnummer) {
                    Some(Ok(opplysninger)) => Some((periode.id, opplysninger.clone())),
                    Some(Err(err)) => {
                        tracing::error!(
                            periode_id = %periode.id,
                            "Feil ved utleding av fakta: {err}",
                        );
                        None
                    }
                    None => {
                        tracing::error!(
                            periode_id = %periode.id,
                            "Ingen PDL data for periode",
                        );
                        None
                    }
                },
            )
            .collect();

        if batch.is_empty() {
            return Ok(());
        }

        let periode_ids: Vec<Uuid> = batch.iter().map(|(id, _)| *id).collect();
        let tidspunkt = chrono::Utc::now();

        let oppdaterte =
            skriv_status_batch(&mut tx, &periode_ids, &Status::Ubehandlet, &tidspunkt).await?;
        tracing::info!("{} perioder oppdatert til Ubehandlet", oppdaterte.len());
        skriv_pdl_info_batch(&mut tx, batch).await?;
        tx.commit().await?;
        Ok(())
    }
}
