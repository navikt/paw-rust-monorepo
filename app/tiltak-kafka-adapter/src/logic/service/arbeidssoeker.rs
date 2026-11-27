use crate::config::SynkConfig;
use crate::model::dao::tiltak_synk::{self, Betingelse};
use crate::model::status::SynkStatus;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Default, PartialEq)]
pub struct Gjenoppretting {
    pub avbrutt: u64,
    pub til_venter: u64,
    pub hengende_aktive: u64,
}

pub struct ArbeidssoekerService {
    pg_pool: PgPool,
    config: SynkConfig,
}

impl ArbeidssoekerService {
    pub fn new(pg_pool: PgPool, config: SynkConfig) -> Arc<Self> {
        Arc::new(Self { pg_pool, config })
    }

    #[tracing::instrument(skip_all)]
    pub async fn init_manglende(&self, synk_id: &Uuid) -> anyhow::Result<u64> {
        let mut tx = self.pg_pool.begin().await?;
        let antall = tiltak_synk::insert_manglende(&mut tx, synk_id).await?;
        tx.commit().await?;
        Ok(antall)
    }

    #[tracing::instrument(skip_all)]
    pub async fn prosesser_feilede(&self) -> anyhow::Result<Gjenoppretting> {
        let grense = self.config.synk_interval_grensed()?;
        let mut tx = self.pg_pool.begin().await?;
        let avbrutt = tiltak_synk::update_status(
            &mut tx,
            &SynkStatus::Feilet,
            &SynkStatus::Avbrutt,
            &Betingelse::AntallFeilMinst(self.config.maks_feil_per_rad),
            false,
        )
        .await?;
        let til_venter = tiltak_synk::update_status(
            &mut tx,
            &SynkStatus::Feilet,
            &SynkStatus::Venter,
            &Betingelse::AntallFeilUnder(self.config.maks_feil_per_rad),
            false,
        )
        .await?;
        let hengende_aktive = tiltak_synk::update_status(
            &mut tx,
            &SynkStatus::Aktiv,
            &SynkStatus::Venter,
            &Betingelse::StatusEndretFoer(grense),
            false,
        )
        .await?;
        tx.commit().await?;
        Ok(Gjenoppretting {
            avbrutt,
            til_venter,
            hengende_aktive,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn prosesser_avbrutte(&self) -> anyhow::Result<u64> {
        self.prosesser_for_status(SynkStatus::Avbrutt).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn prosesser_ferdige(&self) -> anyhow::Result<u64> {
        self.prosesser_for_status(SynkStatus::Ferdig).await
    }

    async fn prosesser_for_status(&self, status: SynkStatus) -> anyhow::Result<u64> {
        let grense = self.config.synk_interval_grensed()?;
        let mut tx = self.pg_pool.begin().await?;
        let antall = tiltak_synk::update_status(
            &mut tx,
            &status,
            &SynkStatus::Venter,
            &Betingelse::SistKjoertFoer(grense),
            true,
        )
        .await?;
        tx.commit().await?;
        Ok(antall)
    }

    pub async fn sett_ferdig(&self, periode_id: &Uuid, synk_id: &Uuid) -> anyhow::Result<u64> {
        self.sett_resultat(periode_id, synk_id, SynkStatus::Ferdig)
            .await
    }

    pub async fn sett_feilet(&self, periode_id: &Uuid, synk_id: &Uuid) -> anyhow::Result<u64> {
        self.sett_resultat(periode_id, synk_id, SynkStatus::Feilet)
            .await
    }

    async fn sett_resultat(
        &self,
        periode_id: &Uuid,
        synk_id: &Uuid,
        status: SynkStatus,
    ) -> anyhow::Result<u64> {
        let mut tx = self.pg_pool.begin().await?;
        let antall = tiltak_synk::update_resultat(&mut tx, periode_id, synk_id, &status).await?;
        tx.commit().await?;
        Ok(antall)
    }

    #[tracing::instrument(skip_all)]
    pub async fn frigi_aktive(&self, synk_id: &Uuid) -> anyhow::Result<u64> {
        let mut tx = self.pg_pool.begin().await?;
        let antall = tiltak_synk::update_status(
            &mut tx,
            &SynkStatus::Aktiv,
            &SynkStatus::Venter,
            &Betingelse::SynkId(*synk_id),
            false,
        )
        .await?;
        tx.commit().await?;
        Ok(antall)
    }
}
