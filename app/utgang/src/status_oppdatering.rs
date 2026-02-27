use std::{collections::HashMap, num::NonZeroU16, ops::Deref, sync::Arc};

use crate::{
    db_read_ops::{hent_opplysninger, hent_periode_metadata, hent_sist_oppdatert_foer},
    db_write_ops::{skriv_pdl_info, skriv_status},
    pdl::pdl_query::PDLClient,
    vo::{periode_metadata_rad::PeriodeMetadata, status::Status},
};
use anyhow::Result;
use chrono::DateTime;
use futures::future::join_all;
use interne_hendelser::vo::Opplysning;
use pdl_graphql::pdl::PdlPerson;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
pub struct StatusOppdatering {
    inner: Arc<StatusOppdateringRef>,
}

struct StatusOppdateringRef {
    pg_pool: PgPool,
    pdl_client: PDLClient,
    batch_size: NonZeroU16,
}

impl StatusOppdatering {
    pub fn new(pg_pool: PgPool, pdl_client: PDLClient, batch_size: NonZeroU16) -> Self {
        Self {
            inner: Arc::new(StatusOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
            }),
        }
    }

    pub async fn kjoer_oppdatering(&self) -> Result<()> {
        let pg_pool = &self.inner.pg_pool;
        let pdl_client = &self.inner.pdl_client;
        let batch_size = &self.inner.batch_size;
        let mut tx = pg_pool.begin().await?;
        let skal_oppdateres =
            hent_sist_oppdatert_foer(&mut tx, &chrono::Utc::now(), batch_size).await?;
        let mut periode_metadata: Vec<PeriodeMetadata> = Vec::with_capacity(skal_oppdateres.len());
        for e in &skal_oppdateres {
            let periode_metadata_rad = hent_periode_metadata(&mut tx, &e.id).await?;
            periode_metadata.push(periode_metadata_rad);
        }
        tx.commit().await?;
        let ident_periode_map: HashMap<String, Uuid> = periode_metadata
            .iter()
            .map(|metadata| (metadata.identitetsnummer.clone(), metadata.periode_id))
            .collect();
        let identiteter: Vec<String> = ident_periode_map.keys().cloned().collect();
        let pdl_info = pdl_client.perform_hent_person_bolk(identiteter).await?;
        let opplysninger: Vec<(&Uuid, Vec<Opplysning>)> = pdl_info
            .iter()
            .filter_map(|pdl_info| {
                ident_periode_map
                    .get(&pdl_info.ident)
                    .map(|periode_id| (periode_id, pdl_info))
            })
            .map(|(periode_id, pdl_info)| {
                let opplysninger: Vec<Opplysning> = todo!();
                (periode_id, opplysninger)
            })
            .collect();
        let mut tx = pg_pool.begin().await?;
        for (periode_id, opplysninger) in opplysninger {
            skriv_pdl_info(&mut tx, periode_id, opplysninger).await?;
            skriv_status(
                &mut tx,
                periode_id,
                &Status::Ubehandlet,
                &chrono::Utc::now(),
            )
            .await?;
        }
        Ok(())
    }
}
