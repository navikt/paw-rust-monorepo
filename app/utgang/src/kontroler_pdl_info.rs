use std::{collections::HashMap, num::NonZeroU16};

use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::regler::regelsett::{EvalueringsResultat, Regelsett};
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
        let klar_for_kontroll: Vec<(usize, KlarForKontrollRad)> =
            hent_klar_for_kontroll(&mut tx, &self.batch_size)
                .await?
                .into_iter()
                .enumerate()
                .collect();
        let param_list: Vec<((usize, InfoType), Vec<Opplysning>)> = klar_for_kontroll
            .iter()
            .flat_map(|(index, rad)| {
                let forrige = rad
                    .forrige_pdl_opplysninger
                    .as_ref()
                    .map(|opplysninger| ((*index, InfoType::Forrige), opplysninger.clone()));
                let startet = rad
                    .startet_opplysninger
                    .as_ref()
                    .map(|opplysninger| ((*index, InfoType::Initiell), opplysninger.clone()));
                [
                    Some(((*index, InfoType::Gjeldende), rad.opplysninger.clone())),
                    forrige,
                    startet,
                ]
                .into_iter()
                .flatten()
            })
            .collect();
        let resultater: HashMap<(usize, InfoType), EvalueringsResultat> = self
            .regelsett
            .evaluer_liste(&param_list)
            .into_iter()
            .map(|(k, v)| (*k, v))
            .collect();
        let resultat: Vec<_> = klar_for_kontroll
            .iter()
            .map(|(index, _rad)| {
                let gjeldende_resultat = resultater.get(&(*index, InfoType::Gjeldende)).cloned();
                let forrige_resultat = resultater.get(&(*index, InfoType::Forrige)).cloned();
                let initiell_resultat = resultater.get(&(*index, InfoType::Initiell)).cloned();
                sjekk_status(gjeldende_resultat, forrige_resultat, initiell_resultat)
            })
            .collect();
        drop(resultat);
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum InfoType {
    Initiell,
    Forrige,
    Gjeldende,
}

#[derive(Debug, PartialEq, Eq)]
pub enum KontrollStatus {
    IngenEndring,
    Endret(EvalueringsResultat),
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SjekkFeil {
    #[error("Mangler gjeldende evalueringsresultat")]
    ManglerGjeldende,
    #[error("Mangler initiell evalueringsresultat")]
    ManglerInitiell,
}

pub fn sjekk_status(
    gjeldende: Option<EvalueringsResultat>,
    forrige: Option<EvalueringsResultat>,
    initiell: Option<EvalueringsResultat>,
) -> Result<KontrollStatus, SjekkFeil> {
    let gjeldende = gjeldende.ok_or(SjekkFeil::ManglerGjeldende)?;
    let initiell = initiell.ok_or(SjekkFeil::ManglerInitiell)?;
    let forrige = forrige.unwrap_or(initiell);
    if forrige == gjeldende {
        Ok(KontrollStatus::IngenEndring)
    } else {
        Ok(KontrollStatus::Endret(gjeldende))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regler_arbeidssoeker::regler::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};
    use regler_arbeidssoeker::regler::regel_id::RegelId;

    fn godkjent() -> EvalueringsResultat {
        Ok(vec![GrunnlagForGodkjenning {
            regel_id: RegelId::Over18AarOgBosattEtterFregLoven,
            opplysninger: vec![],
        }])
    }

    fn avvist() -> EvalueringsResultat {
        Err(vec![Problem {
            regel_id: RegelId::IkkeFunnet,
            opplysninger: vec![],
            kind: ProblemKind::SkalAvvises,
        }])
    }

    #[test]
    fn ingen_endring_nar_gjeldende_lik_forrige() {
        let result = sjekk_status(Some(godkjent()), Some(godkjent()), Some(godkjent()));
        assert_eq!(result, Ok(KontrollStatus::IngenEndring));
    }

    #[test]
    fn ingen_endring_bruker_initiell_som_forrige_naar_forrige_mangler() {
        let result = sjekk_status(Some(godkjent()), None, Some(godkjent()));
        assert_eq!(result, Ok(KontrollStatus::IngenEndring));
    }

    #[test]
    fn endring_naar_gjeldende_ulik_forrige() {
        let result = sjekk_status(Some(avvist()), Some(godkjent()), Some(godkjent()));
        assert_eq!(result, Ok(KontrollStatus::Endret(avvist())));
    }

    #[test]
    fn feil_naar_gjeldende_mangler() {
        let result = sjekk_status(None, None, Some(godkjent()));
        assert!(matches!(result, Err(SjekkFeil::ManglerGjeldende)));
    }

    #[test]
    fn feil_naar_initiell_mangler() {
        let result = sjekk_status(Some(godkjent()), None, None);
        assert!(matches!(result, Err(SjekkFeil::ManglerInitiell)));
    }
}









