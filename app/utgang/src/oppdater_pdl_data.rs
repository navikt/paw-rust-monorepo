use std::{collections::HashMap, num::NonZeroU16, sync::Arc};

use crate::{
    dao::{
        perioder::{hent_perioder_eldre_enn, oppdater_trenger_kontroll},
        utgang_hendelse::{Input, InternUtgangHendelse},
        utgang_hendelser_logg::{PeriodeHendelseData, hent_metadata_og_siste_pdl, skriv_hendelser},
    },
    domain::utgang_hendelse_type::UtgangHendelseType::PdlDataEndret,
    pdl::pdl_query::PDLClient,
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use interne_hendelser::vo::{BrukerType, Opplysninger};
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
    pub async fn kjoer_oppdatering(&self, gjeldene_tidspunkt: DateTime<Utc>) -> Result<()> {
        tracing::info!("Starter oppdatering av PDL data");
        let vannmerke = gjeldene_tidspunkt - self.inner.data_gyldighet;
        let mut tx = self.inner.pg_pool.begin().await?;
        let trenger_oppdatering =
            hent_perioder_eldre_enn(&mut tx, vannmerke, self.inner.batch_size).await?;
        if trenger_oppdatering.len() == 0 {
            return Ok(());
        }
        let ident_map: HashMap<Identitetsnummer, ArbeidssoekerperiodeId> = trenger_oppdatering
            .iter()
            .map(|periode| (periode.identitetsnummer.clone(), periode.id.clone()))
            .collect();
        let identitetsnummer: Vec<Identitetsnummer> = trenger_oppdatering
            .iter()
            .map(|periode| periode.identitetsnummer.clone())
            .collect();
        let periode_ider: Vec<ArbeidssoekerperiodeId> = trenger_oppdatering
            .iter()
            .map(|periode| periode.id.clone())
            .collect();
        let pdl_data = self
            .hent_og_koble_pdl_data(identitetsnummer, trenger_oppdatering.len())
            .await?;

        let gjeldende_opplysninger = utled_fakta(pdl_data);
        let gjeldende_data = hent_metadata_og_siste_pdl(&mut tx, &periode_ider).await?;
        let endret = finn_endrede_hendelser(
            gjeldende_opplysninger,
            gjeldende_data,
            &ident_map,
            gjeldene_tidspunkt,
        );
        skriv_hendelser(&mut tx, &endret).await?;
        let endrede_perioder: Vec<ArbeidssoekerperiodeId> =
            endret.into_iter().map(|e| e.into_periode_id()).collect();
        oppdater_trenger_kontroll(&mut tx, &endrede_perioder, true).await?;
        Ok(())
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
#[instrument(skip(gjeldende_opplysninger, gjeldende_data, ident_map))]
fn finn_endrede_hendelser(
    gjeldende_opplysninger: Vec<(Identitetsnummer, anyhow::Result<Opplysninger>)>,
    mut gjeldende_data: HashMap<ArbeidssoekerperiodeId, PeriodeHendelseData>,
    ident_map: &HashMap<Identitetsnummer, ArbeidssoekerperiodeId>,
    gjeldene_tidspunkt: DateTime<Utc>,
) -> Vec<InternUtgangHendelse<Input>> {
    gjeldende_opplysninger
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
                Some(InternUtgangHendelse::new(
                    PdlDataEndret,
                    periode_id.clone(),
                    gjeldene_tidspunkt,
                    BrukerType::System,
                    Some(opplysninger),
                ))
            } else {
                None
            }
        })
        .collect()
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
    use std::collections::HashMap;

    use chrono::Utc;
    use interne_hendelser::vo::{BrukerType, Opplysning, Opplysninger};
    use pdl_graphql::pdl::hent_person_bolk::HentPersonBolkHentPersonBolk;
    use uuid::Uuid;

    use crate::dao::utgang_hendelse::{InternUtgangHendelse, Output};
    use crate::dao::utgang_hendelser_logg::PeriodeHendelseData;
    use crate::domain::utgang_hendelse_type::UtgangHendelseType;
    use types::{
        arbeidssoekerperiode_id::ArbeidssoekerperiodeId, identitetsnummer::Identitetsnummer,
    };

    use super::{finn_endrede_hendelser, koble_ident_med_person};

    fn ident(fnr: &str) -> Identitetsnummer {
        Identitetsnummer::new(fnr.to_string()).expect("ugyldig testident")
    }

    fn periode_id() -> ArbeidssoekerperiodeId {
        ArbeidssoekerperiodeId(Uuid::new_v4())
    }

    fn opplysninger_a() -> Opplysninger {
        Opplysninger::new(vec![Opplysning::ErOver18Aar, Opplysning::IkkeAnsatt])
    }

    fn opplysninger_b() -> Opplysninger {
        Opplysninger::new(vec![Opplysning::ErUnder18Aar])
    }

    fn hendelse_med_opplysninger(
        pid: ArbeidssoekerperiodeId,
        opl: Option<Opplysninger>,
    ) -> InternUtgangHendelse<Output> {
        InternUtgangHendelse::from_db_row(
            1,
            UtgangHendelseType::MetadataMottatt,
            pid,
            Utc::now(),
            BrukerType::System,
            opl,
        )
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

    // --- koble_ident_med_person ---

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

    // --- finn_endrede_hendelser ---

    #[test]
    fn returnerer_hendelse_naar_opplysninger_er_endret() {
        let pid = periode_id();
        let id = ident("12345678901");
        let ident_map = HashMap::from([(id.clone(), pid.clone())]);
        let gjeldende_data = HashMap::from([(
            pid.clone(),
            PeriodeHendelseData {
                metadata_mottatt: hendelse_med_opplysninger(pid.clone(), Some(opplysninger_a())),
                siste_pdl_data_endret: None,
            },
        )]);

        let resultat = finn_endrede_hendelser(
            vec![(id, Ok(opplysninger_b()))],
            gjeldende_data,
            &ident_map,
            Utc::now(),
        );

        assert_eq!(resultat.len(), 1);
    }

    #[test]
    fn returnerer_ingenting_naar_opplysninger_er_like() {
        let pid = periode_id();
        let id = ident("12345678901");
        let ident_map = HashMap::from([(id.clone(), pid.clone())]);
        let gjeldende_data = HashMap::from([(
            pid.clone(),
            PeriodeHendelseData {
                metadata_mottatt: hendelse_med_opplysninger(pid.clone(), Some(opplysninger_a())),
                siste_pdl_data_endret: None,
            },
        )]);

        let resultat = finn_endrede_hendelser(
            vec![(id, Ok(opplysninger_a()))],
            gjeldende_data,
            &ident_map,
            Utc::now(),
        );

        assert!(resultat.is_empty());
    }

    #[test]
    fn bruker_siste_pdl_data_endret_fremfor_metadata() {
        let pid = periode_id();
        let id = ident("12345678901");
        let ident_map = HashMap::from([(id.clone(), pid.clone())]);
        let gjeldende_data = HashMap::from([(
            pid.clone(),
            PeriodeHendelseData {
                metadata_mottatt: hendelse_med_opplysninger(pid.clone(), Some(opplysninger_b())),
                siste_pdl_data_endret: Some(hendelse_med_opplysninger(
                    pid.clone(),
                    Some(opplysninger_a()),
                )),
            },
        )]);

        let resultat = finn_endrede_hendelser(
            vec![(id, Ok(opplysninger_a()))],
            gjeldende_data,
            &ident_map,
            Utc::now(),
        );

        assert!(
            resultat.is_empty(),
            "skal matche siste_pdl_data_endret, ikke metadata"
        );
    }

    #[test]
    fn utelater_ident_uten_matchende_periode_id() {
        let id = ident("12345678901");
        let resultat = finn_endrede_hendelser(
            vec![(id, Ok(opplysninger_a()))],
            HashMap::new(),
            &HashMap::new(),
            Utc::now(),
        );
        assert!(resultat.is_empty());
    }

    #[test]
    fn utelater_ident_uten_lagret_pdl_data() {
        let pid = periode_id();
        let id = ident("12345678901");
        let ident_map = HashMap::from([(id.clone(), pid.clone())]);

        let resultat = finn_endrede_hendelser(
            vec![(id, Ok(opplysninger_a()))],
            HashMap::new(),
            &ident_map,
            Utc::now(),
        );
        assert!(resultat.is_empty());
    }

    #[test]
    fn utelater_ved_feil_i_opplysninger() {
        let pid = periode_id();
        let id = ident("12345678901");
        let ident_map = HashMap::from([(id.clone(), pid.clone())]);
        let gjeldende_data = HashMap::from([(
            pid.clone(),
            PeriodeHendelseData {
                metadata_mottatt: hendelse_med_opplysninger(pid.clone(), Some(opplysninger_a())),
                siste_pdl_data_endret: None,
            },
        )]);

        let resultat = finn_endrede_hendelser(
            vec![(id, Err(anyhow::anyhow!("testfeil")))],
            gjeldende_data,
            &ident_map,
            Utc::now(),
        );
        assert!(resultat.is_empty());
    }
}
