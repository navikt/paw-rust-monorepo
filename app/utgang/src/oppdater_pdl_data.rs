use std::{collections::HashMap, num::NonZeroU16, sync::Arc};

use crate::{
    db_read_ops::{hent_periode_metadata, hent_sist_oppdatert_foer},
    db_write_ops::skriv_pdl_info,
    pdl::pdl_query::PDLClient,
    vo::{periode_metadata_rad::PeriodeMetadata, status::Status},
};
use anyhow::Result;
use chrono::Duration;
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::fakta::{UtledeFakta, person_fakta::UtledePersonFakta};
use sqlx::PgPool;
use tracing::instrument;

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
        let skal_oppdateres = hent_sist_oppdatert_foer(
            &mut tx,
            &sist_oppdatert_foer,
            &[Status::Ok, Status::Avvist],
            batch_size,
        )
        .await?;
        tracing::info!("{} perioder skal oppdateres", skal_oppdateres.len());
        let mut periode_metadata: Vec<PeriodeMetadata> = Vec::with_capacity(skal_oppdateres.len());
        for e in &skal_oppdateres {
            let periode_metadata_rad = hent_periode_metadata(&mut tx, &e.id).await?;
            periode_metadata.push(periode_metadata_rad);
        }
        let periode_metadata = periode_metadata;
        tx.commit().await?;
        tracing::info!("Hentet metadata for {} perioder", periode_metadata.len());
        if periode_metadata.is_empty() {
            return Ok(());
        }
        let identitetsnummer: Vec<String> = periode_metadata
            .iter()
            .map(|pm| pm.identitetsnummer.clone())
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
        let mut tx = pg_pool.begin().await?;
        for periode in periode_metadata {
            let identitetsnummer = periode.identitetsnummer;
            let periode_id = periode.periode_id;
            let opplysninger = ident_til_person.get(&identitetsnummer);
            match opplysninger {
                Some(Ok(opplysninger)) => {
                    skriv_pdl_info(&mut tx, &periode_id, opplysninger.clone()).await?;
                }
                Some(Err(err)) => {
                    tracing::error!(
                        "Feil ved utleding av fakta for periode: {} : {}",
                        periode_id,
                        err
                    );
                }
                None => {
                    tracing::error!("Fant ingen PDL data for periode: {}", periode_id);
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }
}
