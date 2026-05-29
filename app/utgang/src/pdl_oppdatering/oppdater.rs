use std::{collections::HashMap, num::NonZeroU16, sync::Arc};

use crate::{dao::les_periode::hent_utdaterte_perioder, pdl::pdl_query::PDLClient};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use pdl_graphql::pdl::{Person, hent_person_bolk::HentPersonBolkHentPersonBolk};
use regler_arbeidssoeker::fakta::person_fakta::utled_fakta;
use sqlx::PgPool;
use tracing::instrument;
use types::identitetsnummer::Identitetsnummer;

#[derive(Clone)]
pub struct PdlDataOppdatering {
    inner: Arc<PdlDataOppdateringRef>,
}

struct PdlDataOppdateringRef {
    pg_pool: PgPool,
    pdl_client: PDLClient,
    batch_size: NonZeroU16,
    data_gyldighet: Duration,
}

impl PdlDataOppdatering {
    pub fn new(
        pg_pool: PgPool,
        pdl_client: PDLClient,
        batch_size: NonZeroU16,
        data_gyldighet: Duration,
    ) -> Self {
        Self {
            inner: Arc::new(PdlDataOppdateringRef {
                pg_pool,
                pdl_client,
                batch_size,
                data_gyldighet,
            }),
        }
    }
    #[instrument(skip(self))]
    pub async fn kjoer_oppdatering(&self, gjeldene_tidspunkt: DateTime<Utc>) -> Result<bool> {
        tracing::info!("Starter oppdatering av PDL data");
        let vannmerke = gjeldene_tidspunkt - self.inner.data_gyldighet;
        let mut tx = self.inner.pg_pool.begin().await?;
        let trenger_oppdatering =
            hent_utdaterte_perioder(&mut tx, vannmerke, self.inner.batch_size).await?;
        let identitetsnummer: Vec<Identitetsnummer> = trenger_oppdatering
            .iter()
            .map(|periode| periode.identitetsnummer.clone())
            .collect();
        let pdl_data = self
            .inner
            .pdl_client
            .perform_hent_person_bolk(identitetsnummer)
            .await?;
        let identitetsnummer_og_person = generer_identitetsnummer(pdl_data);
        let identitetsnummer_og_opplysninger = utled_fakta(identitetsnummer_og_person);
        Ok(false)
    }
}

fn generer_identitetsnummer(
    pdl_data: impl IntoIterator<Item = HentPersonBolkHentPersonBolk>,
) -> Vec<(Identitetsnummer, Person)> {
    pdl_data
        .into_iter()
        .filter_map(|pdl| {
            let pdl_person = pdl.person;
            let identitetsnummer = Identitetsnummer::new(pdl.ident);
            let res = identitetsnummer.zip(pdl_person);
            if res.is_none() {
                tracing::warn!("Ugyldig identitetsnummer i PDL data");
            }
            res
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use interne_hendelser::vo::{Opplysning, Opplysninger};
    use pdl_graphql::pdl::hent_person_bolk::HentPersonBolkHentPersonBolk;
    use types::identitetsnummer::Identitetsnummer;

    fn ident(fnr: &str) -> Identitetsnummer {
        Identitetsnummer::new(fnr.to_string()).expect("ugyldig testident")
    }

    fn opplysninger_a() -> Opplysninger {
        Opplysninger::new(vec![Opplysning::ErOver18Aar, Opplysning::IkkeAnsatt])
    }

    fn opplysninger_b() -> Opplysninger {
        Opplysninger::new(vec![Opplysning::ErUnder18Aar])
    }

    fn pdl_bolk(fnr: &str, med_person: bool) -> HentPersonBolkHentPersonBolk {
        HentPersonBolkHentPersonBolk {
            ident: fnr.to_string(),
            person: if med_person {
                Some(
                    pdl_graphql::pdl::hent_person_bolk::HentPersonBolkHentPersonBolkPerson {
                        foedselsdato: vec![],
                        statsborgerskap: vec![],
                        opphold: vec![],
                        folkeregisterpersonstatus: vec![],
                        bostedsadresse: vec![],
                        innflytting_til_norge: vec![],
                        utflytting_fra_norge: vec![],
                    },
                )
            } else {
                None
            },
            code: "ok".to_string(),
        }
    }
}
