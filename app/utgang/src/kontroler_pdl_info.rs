use std::num::NonZeroU16;

use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::regler::regelsett::{self, Regelsett};
use utgang::{db_read_ops::hent_klar_for_kontroll, vo::klar_for_kontroll_rad::KlarForKontrollRad};

pub struct KontrolerKlarForKontroll {
    batch_size: NonZeroU16,
    pg_pool: sqlx::PgPool,
    intervall: chrono::Duration,
    regelsett: Regelsett,
}

impl KontrolerKlarForKontroll {
    pub fn new(
        batch_size: NonZeroU16,
        pg_pool: sqlx::PgPool,
        intervall: chrono::Duration,
        regelsett: Regelsett,
    ) -> Self {
        Self {
            batch_size,
            pg_pool,
            intervall,
            regelsett,
        }
    }

    pub async fn kontroler_klar_for_kontroll(&self) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        let klar_for_kontroll = hent_klar_for_kontroll(&mut tx, &self.batch_size)
            .await?
            .into_iter()
            .filter(
                |rad| matches!(&rad.forrige_pdl_opplysninger, Some(o) if o != &rad.opplysninger),
            )
            .collect::<Vec<KlarForKontrollRad>>();
        let gjeldene: Vec<(KlarForKontrollRad, Vec<Opplysning>)> = klar_for_kontroll
            .into_iter()
            .map(|rad| {
                let opplysninger = rad.opplysninger.clone();
                (rad, opplysninger)
            })
            .collect();
        let kontroll_resultat = self.regelsett.evaluer_liste(&gjeldene);

        Ok(())
    }
}
