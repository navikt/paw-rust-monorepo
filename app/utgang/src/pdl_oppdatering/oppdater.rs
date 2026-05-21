use std::{collections::HashMap, num::NonZeroU16, sync::Arc};

use crate::dao::perioder::{hent_perioder_eldre_enn, oppdater_pdl_opplysninger, oppdater_sist_oppdatert};
use crate::pdl::pdl_query::PDLClient;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use interne_hendelser::vo::Opplysninger;
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
            hent_perioder_eldre_enn(&mut tx, vannmerke, self.inner.batch_size).await?;
        let antall = trenger_oppdatering.len();
        if antall == 0 {
            return Ok(false);
        }
        let ident_map: HashMap<Identitetsnummer, ArbeidssoekerperiodeId> = trenger_oppdatering
            .iter()
            .map(|p| (p.identitetsnummer.clone(), p.id.clone()))
            .collect();
        let gjeldende_map: HashMap<ArbeidssoekerperiodeId, Option<Opplysninger>> =
            trenger_oppdatering
                .iter()
                .map(|p| (p.id.clone(), p.gjeldende_opplysninger.clone()))
                .collect();
        let identitetsnummer: Vec<Identitetsnummer> = trenger_oppdatering
            .iter()
            .map(|p| p.identitetsnummer.clone())
            .collect();
        let uendrede_perioder: Vec<ArbeidssoekerperiodeId> = trenger_oppdatering
            .iter()
            .map(|p| p.id.clone())
            .collect();

        let pdl_data = self.hent_og_koble_pdl_data(identitetsnummer, antall).await?;
        let nye_opplysninger = utled_fakta(pdl_data);

        let mut oppdaterte: Vec<ArbeidssoekerperiodeId> = Vec::new();
        for (ident, opl_result) in nye_opplysninger {
            let opl = match opl_result {
                Ok(o) => o,
                Err(e) => {
                    tracing::error!("Feil ved utledning av opplysninger: {:?}", e);
                    continue;
                }
            };
            let Some(periode_id) = ident_map.get(&ident) else {
                continue;
            };
            let lagret = gjeldende_map.get(periode_id).and_then(|o| o.as_ref());
            let er_endret = lagret.map_or(true, |l| *l != opl);
            if er_endret {
                oppdater_pdl_opplysninger(&mut tx, periode_id, &opl, gjeldene_tidspunkt).await?;
                oppdaterte.push(periode_id.clone());
            }
        }

        let uendrede: Vec<ArbeidssoekerperiodeId> = uendrede_perioder
            .into_iter()
            .filter(|id| !oppdaterte.contains(id))
            .collect();
        oppdater_sist_oppdatert(&mut tx, &uendrede, gjeldene_tidspunkt).await?;
        tx.commit().await?;
        tracing::info!(
            "PDL oppdatering ferdig: {} endret, {} uendret",
            oppdaterte.len(),
            uendrede.len()
        );
        Ok(antall > 0)
    }

    #[instrument(skip(self, identitetsnummer))]
    async fn hent_og_koble_pdl_data(
        &self,
        identitetsnummer: Vec<Identitetsnummer>,
        antall_perioder: usize,
    ) -> Result<Vec<(Identitetsnummer, Person)>> {
        let pdl_data = self
            .inner
            .pdl_client
            .perform_hent_person_bolk(identitetsnummer.clone())
            .await?;
        let pdl_data = koble_ident_med_person(identitetsnummer, pdl_data);
        let mut manglende_data = 0_u16;
        let pdl_data = pdl_data
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
        Ok(pdl_data)
    }
}

#[instrument(skip(identietsnummer, pdl_data))]
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

#[cfg(test)]
mod tests {
    use interne_hendelser::vo::{Opplysning, Opplysninger};
    use pdl_graphql::pdl::hent_person_bolk::HentPersonBolkHentPersonBolk;
    use types::identitetsnummer::Identitetsnummer;

    use super::koble_ident_med_person;

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

    #[test]
    fn kobler_ident_med_matchende_person() {
        let fnr = "12345678901";
        let result = koble_ident_med_person(vec![ident(fnr)], vec![pdl_bolk(fnr, true)]);
        assert_eq!(result.len(), 1);
        assert!(result[0].1.is_some());
    }

    #[test]
    fn returnerer_none_for_ident_uten_pdl_treff() {
        let fnr = "12345678901";
        let result = koble_ident_med_person(vec![ident(fnr)], vec![]);
        assert_eq!(result.len(), 1);
        assert!(result[0].1.is_none());
    }

    #[test]
    fn returnerer_none_naar_pdl_mangler_person() {
        let fnr = "12345678901";
        let result = koble_ident_med_person(vec![ident(fnr)], vec![pdl_bolk(fnr, false)]);
        assert_eq!(result.len(), 1);
        assert!(result[0].1.is_none());
    }

    #[test]
    fn pdl_data_uten_matchende_ident_ignoreres() {
        let result = koble_ident_med_person(
            vec![ident("12345678901")],
            vec![pdl_bolk("09876543210", true)],
        );
        assert_eq!(result.len(), 1);
        assert!(result[0].1.is_none());
    }

    #[test]
    fn bevarer_rekkefølge_fra_input_ident_liste() {
        let fnr1 = "12345678901";
        let fnr2 = "10987654321";
        let result = koble_ident_med_person(
            vec![ident(fnr1), ident(fnr2)],
            vec![pdl_bolk(fnr2, true), pdl_bolk(fnr1, true)],
        );
        assert_eq!(result[0].0, ident(fnr1));
        assert_eq!(result[1].0, ident(fnr2));
    }
}
