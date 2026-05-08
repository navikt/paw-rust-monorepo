use std::{collections::HashMap, num::NonZeroU16, sync::Arc};

use crate::{
    dao::{
        perioder::hent_perioder_eldre_enn,
        utgang_hendelse::{Input, InternUtgangHendelse},
        utgang_hendelser_logg::{hent_metadata_og_siste_pdl, skriv_hendelser},
    },
    domain::utgang_hendelse_type::UtgangHendelseType::PdlDataEndret,
    pdl::pdl_query::PDLClient,
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use interne_hendelser::vo::BrukerType;
use pdl_graphql::pdl::{Person, hent_person_bolk::HentPersonBolkHentPersonBolk};
use regler_arbeidssoeker::fakta::person_fakta::utled_fakta;
use sqlx::PgPool;
use tracing::instrument;
use types::{arbeidssoekerperiode_id::ArbeidssoekerperiodeId, identitetsnummer::Identitetsnummer};

#[derive(Clone)]
pub struct PdlDataOppdatering {
    inner: Arc<PdlDataOppdateringRef>,
}

struct PdlDataOppdateringRef {
    pg_pool: PgPool,
    pdl_client: PDLClient,
    batch_size: NonZeroU16,
    intervall: Duration,
    data_gyldighet: Duration,
}

impl PdlDataOppdatering {
    pub fn new(
        pg_pool: PgPool,
        pdl_client: PDLClient,
        batch_size: NonZeroU16,
        intervall: Duration,
        data_gyldighet: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(PdlDataOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
                intervall,
                data_gyldighet,
            }),
        }
    }
    #[instrument(skip(self))]
    pub async fn kjoer_oppdatering(&self, gjeldene_tidspunkt: DateTime<Utc>) -> Result<()> {
        tracing::info!("Starter oppdatering av PDL data");
        let vannmerke = gjeldene_tidspunkt - self.inner.data_gyldighet;
        let mut tx = self.inner.pg_pool.begin().await?;
        let trenger_oppdatering =
            hent_perioder_eldre_enn(&mut tx, vannmerke, self.inner.batch_size).await?;
        let ident_map: HashMap<Identitetsnummer, ArbeidssoekerperiodeId> = trenger_oppdatering
            .iter()
            .map(|periode| (periode.identitetsnummer.clone(), periode.id.clone()))
            .collect();
        let antall_perioder = trenger_oppdatering.len();
        let identitetsnummer: Vec<Identitetsnummer> = trenger_oppdatering
            .iter()
            .map(|periode| periode.identitetsnummer.clone())
            .collect();
        let periode_ider: Vec<ArbeidssoekerperiodeId> = trenger_oppdatering
            .iter()
            .map(|periode| periode.id.clone())
            .collect();
        let pdl_data = self
            .inner
            .pdl_client
            .perform_hent_person_bolk(identitetsnummer.clone())
            .await?;
        let pdl_data = koble_ident_med_person(identitetsnummer, pdl_data);
        let mut manglende_data = 0_u16;
        let pdl_data: Vec<(Identitetsnummer, Person)> = pdl_data
            .into_iter()
            .filter_map(|(ident, person_opt)| {
                if person_opt.is_none() {
                    manglende_data += 1;
                }
                person_opt.map(|person| (ident, person))
            })
            .collect();
        tracing::info!(
            "Hentet PDL data for {} perioder, mangler data for {} personer",
            antall_perioder,
            manglende_data
        );
        let gjeldende_opplysninger = utled_fakta(pdl_data);
        let mut gjeldende_data = hent_metadata_og_siste_pdl(&mut tx, &periode_ider).await?;
        let endret: Vec<InternUtgangHendelse<Input>> = gjeldende_opplysninger
            .into_iter()
            .filter_map(|(ident, opplysninger)| match opplysninger {
                Ok(opl) => Some((ident, opl)),
                Err(e) => {
                    tracing::error!("Feil ved utledning av opplysninger: {:?}", e);
                    None
                }
            })
            .filter_map(|(ident, opplysninger)| {
                let periode_id = ident_map.get(&ident)?;
                let lagret = gjeldende_data.remove(periode_id)?;
                let siste = lagret
                    .siste_pdl_data_endret
                    .and_then(|e| e.into_opplysninger())
                    .or_else(|| lagret.metadata_mottatt.into_opplysninger())?;
                if opplysninger != siste {
                    let hendelse = InternUtgangHendelse::new(
                        PdlDataEndret,
                        periode_id.clone(),
                        gjeldene_tidspunkt,
                        BrukerType::System,
                        Some(opplysninger),
                    );
                    Some(hendelse)
                } else {
                    None
                }
            })
            .collect();
        skriv_hendelser(&mut tx, endret).await?;
        Ok(())
    }
}

pub fn koble_ident_med_person(
    identietsnummer: Vec<Identitetsnummer>,
    pdl_data: Vec<HentPersonBolkHentPersonBolk>,
) -> Vec<(Identitetsnummer, Option<Person>)> {
    let mut pdl_map: HashMap<Identitetsnummer, HentPersonBolkHentPersonBolk> = pdl_data
        .into_iter()
        .filter_map(|hp| Identitetsnummer::new(hp.ident.clone()).map(|ident| (ident, hp)))
        .collect();
    identietsnummer
        .into_iter()
        .map(|ident| {
            let person = pdl_map.remove(&ident).and_then(|hp| hp.person);
            (ident, person)
        })
        .collect()
}
