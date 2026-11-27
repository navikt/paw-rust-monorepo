use crate::config::SynkConfig;
use crate::kafka::publisher::TiltakshistorikkPublisher;
use crate::logic::batch::{BatchBehandler, BatchSlutt, Kjoering};
use crate::logic::metrics::tiltak_synk as metrics;
use crate::logic::service::arbeidssoeker::ArbeidssoekerService;
use crate::model::dao::tiltak_synk::{self, BatchTiltakSynkRow};
use crate::model::error::SynkFeil;
use mulighetsrommet_client::client::MulighetsrommetClient;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_key_gen_client::model::IdentitetType;
use sqlx::PgPool;
use std::ops::ControlFlow;
use std::sync::Arc;
use uuid::Uuid;

pub struct TiltakSynkJobb<P: TiltakshistorikkPublisher> {
    pg_pool: PgPool,
    config: SynkConfig,
    arbeidssoeker_service: Arc<ArbeidssoekerService>,
    key_gen_client: Arc<PawKeyGenClient>,
    mulighetsrommet_client: MulighetsrommetClient,
    publisher: Arc<P>,
}

impl<P: TiltakshistorikkPublisher + Send + Sync> TiltakSynkJobb<P> {
    pub fn new(
        pg_pool: PgPool,
        config: SynkConfig,
        arbeidssoeker_service: Arc<ArbeidssoekerService>,
        key_gen_client: Arc<PawKeyGenClient>,
        mulighetsrommet_client: MulighetsrommetClient,
        publisher: Arc<P>,
    ) -> Arc<Self> {
        Arc::new(Self {
            pg_pool,
            config,
            arbeidssoeker_service,
            key_gen_client,
            mulighetsrommet_client,
            publisher,
        })
    }

    pub fn config(&self) -> &SynkConfig {
        &self.config
    }

    #[tracing::instrument(skip_all, fields(synk_id = %synk_id))]
    pub async fn run(&self, synk_id: Uuid) -> Kjoering {
        tracing::info!("Starter synkronisering av tiltakshistorikk");
        let service = &self.arbeidssoeker_service;

        match service.init_manglende(&synk_id).await {
            Ok(antall) => tracing::info!(antall, "Steg 1: Opprettet manglende synk-rader"),
            Err(error) => tracing::error!(error = %error, "Steg 1 feilet"),
        }
        match service.prosesser_feilede().await {
            Ok(resultat) => {
                metrics::registrer_avbrutt(resultat.avbrutt);
                tracing::info!(
                    avbrutt = resultat.avbrutt,
                    til_venter = resultat.til_venter,
                    hengende_aktive = resultat.hengende_aktive,
                    "Steg 2: Behandlet feilede og hengende rader"
                )
            }
            Err(error) => tracing::error!(error = %error, "Steg 2 feilet"),
        }
        match service.prosesser_avbrutte().await {
            Ok(antall) => tracing::info!(antall, "Steg 3: Satt AVBRUTT til VENTER"),
            Err(error) => tracing::error!(error = %error, "Steg 3 feilet"),
        }
        match service.prosesser_ferdige().await {
            Ok(antall) => tracing::info!(antall, "Steg 4: Satt FERDIG til VENTER"),
            Err(error) => tracing::error!(error = %error, "Steg 4 feilet"),
        }

        let mut steg5 = Steg5 {
            jobb: self,
            synk_id,
            kjoering: Kjoering::default(),
            feil_teller: 0,
        };
        let resultat =
            for_hver_batch(&self.pg_pool, &synk_id, self.config.batch_size, &mut steg5).await;
        let mut kjoering = steg5.kjoering;
        match resultat {
            Ok(slutt) => kjoering.slutt = Some(slutt),
            Err(error) => tracing::error!(error = %error, "Steg 5 feilet"),
        }

        match service.frigi_aktive(&synk_id).await {
            Ok(antall) => kjoering.frigitt = antall,
            Err(error) => tracing::error!(error = %error, "Kunne ikke frigi aktive rader"),
        }

        tracing::info!(
            synkronisert = kjoering.synkronisert,
            feilet = kjoering.feilet,
            frigitt = kjoering.frigitt,
            slutt = ?kjoering.slutt,
            "Synkronisering av tiltakshistorikk ferdig"
        );
        kjoering
    }

    #[tracing::instrument(skip_all, fields(periode_id = %rad.periode_id))]
    async fn behandle_rad(
        &self,
        synk_id: &Uuid,
        rad: &BatchTiltakSynkRow,
        kjoering: &mut Kjoering,
        feil_teller: &mut u32,
    ) {
        let synk = self.synk_arbeidssoeker(rad).await;
        if let Err(error) = &synk {
            tracing::warn!(error = %error, "Synkronisering av arbeidssøker feilet");
        }
        let oppdatering = match synk {
            Ok(()) => {
                self.arbeidssoeker_service
                    .sett_ferdig(&rad.periode_id, synk_id)
                    .await
            }
            Err(_) => {
                self.arbeidssoeker_service
                    .sett_feilet(&rad.periode_id, synk_id)
                    .await
            }
        };
        match (oppdatering, synk.is_ok()) {
            (Ok(0), _) => {
                tracing::debug!("Raden er slettet eller tatt over, status ble ikke oppdatert")
            }
            (Ok(_), true) => {
                metrics::registrer_synkronisert();
                kjoering.synkronisert += 1;
            }
            (Ok(_), false) => {
                metrics::registrer_feilet();
                kjoering.feilet += 1;
                *feil_teller += 1;
            }
            (Err(error), _) => {
                tracing::error!(error = %error, "Kunne ikke oppdatere status");
                *feil_teller += 1;
            }
        }
    }

    async fn synk_arbeidssoeker(&self, rad: &BatchTiltakSynkRow) -> Result<(), SynkFeil> {
        tokio::time::sleep(self.config.request_grace_time).await;

        let response = self
            .key_gen_client
            .finn_identiteter(rad.identitetsnummer.clone())
            .await
            .map_err(SynkFeil::KeyGen)?;

        let identer: Vec<String> = response
            .identiteter
            .into_iter()
            .filter(|identitet| identitet.identitet_type == IdentitetType::Folkeregisterident)
            .map(|identitet| identitet.identitet)
            .collect();
        if identer.is_empty() {
            return Err(SynkFeil::IngenIdenter);
        }

        tokio::time::sleep(self.config.request_grace_time).await;
        let historikk = self.mulighetsrommet_client.hent_historikk(&identer).await?;

        self.publisher
            .publiser(&rad.periode_id, &historikk)
            .await
            .map_err(SynkFeil::Publisher)
    }
}

struct Steg5<'a, P: TiltakshistorikkPublisher> {
    jobb: &'a TiltakSynkJobb<P>,
    synk_id: Uuid,
    kjoering: Kjoering,
    feil_teller: u32,
}

impl<P: TiltakshistorikkPublisher + Send + Sync> BatchBehandler for Steg5<'_, P> {
    async fn behandle(&mut self, rader: Vec<BatchTiltakSynkRow>) -> ControlFlow<()> {
        for rad in rader {
            self.jobb
                .behandle_rad(
                    &self.synk_id,
                    &rad,
                    &mut self.kjoering,
                    &mut self.feil_teller,
                )
                .await;
            if self.feil_teller >= self.jobb.config.maks_feil_per_kjoering {
                tracing::warn!(
                    feil_teller = self.feil_teller,
                    "Avbryter kjøringen etter for mange feil"
                );
                return ControlFlow::Break(());
            }
        }
        tokio::time::sleep(self.jobb.config.batch_grace_time).await;
        ControlFlow::Continue(())
    }
}

#[tracing::instrument(skip_all)]
pub async fn for_hver_batch(
    pool: &PgPool,
    synk_id: &Uuid,
    batch_stoerrelse: i64,
    behandler: &mut (impl BatchBehandler + Send),
) -> anyhow::Result<BatchSlutt> {
    loop {
        let mut batch_tx = pool.begin().await?;
        let rows = tiltak_synk::select_batch(&mut batch_tx, synk_id, batch_stoerrelse).await?;
        batch_tx.commit().await?;

        if rows.is_empty() {
            return Ok(BatchSlutt::IngenVentende);
        }
        if behandler.behandle(rows).await.is_break() {
            return Ok(BatchSlutt::Stoppet);
        }
    }
}
