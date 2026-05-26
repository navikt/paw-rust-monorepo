use regler_arbeidssoeker::regler::regelsett::EvalueringsResultat;

#[derive(Debug, PartialEq, Eq)]
pub enum KontrollStatus {
    IngenEndring,
    Endret(EvalueringsResultat),
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SjekkFeil {
    #[error("Mangler gjeldende evalueringsresultat")]
    ManglerGjeldende,
}

/// Sammenligner gjeldende evalueringsresultat mot forrige.
/// Hvis forrige mangler (første kontroll) antas ingen endring.
pub fn sjekk_status(
    gjeldende: Option<EvalueringsResultat>,
    forrige: Option<EvalueringsResultat>,
) -> Result<KontrollStatus, SjekkFeil> {
    let gjeldende = gjeldende.ok_or(SjekkFeil::ManglerGjeldende)?;
    let Some(forrige) = forrige else {
        return Ok(KontrollStatus::IngenEndring);
    };
    if forrige == gjeldende {
        Ok(KontrollStatus::IngenEndring)
    } else {
        Ok(KontrollStatus::Endret(gjeldende))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regler_arbeidssoeker::regler::regel_id::RegelId;
    use regler_arbeidssoeker::regler::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};

    fn godkjent() -> EvalueringsResultat {
        EvalueringsResultat::Godkjent {
            grunnlag: vec![GrunnlagForGodkjenning {
                regel_id: RegelId::Over18AarOgBosattEtterFregLoven,
                opplysninger: vec![],
            }],
        }
    }

    fn avvist() -> EvalueringsResultat {
        EvalueringsResultat::Avvist {
            problemer: vec![Problem {
                regel_id: RegelId::IkkeFunnet,
                opplysninger: vec![],
                kind: ProblemKind::SkalAvvises,
            }],
        }
    }

    #[test]
    fn ingen_endring_nar_gjeldende_lik_forrige() {
        let result = sjekk_status(Some(godkjent()), Some(godkjent()));
        assert_eq!(result, Ok(KontrollStatus::IngenEndring));
    }

    #[test]
    fn ingen_endring_naar_forrige_mangler() {
        let result = sjekk_status(Some(godkjent()), None);
        assert_eq!(result, Ok(KontrollStatus::IngenEndring));
    }

    #[test]
    fn endring_naar_gjeldende_ulik_forrige() {
        let result = sjekk_status(Some(avvist()), Some(godkjent()));
        assert_eq!(result, Ok(KontrollStatus::Endret(avvist())));
    }

    #[test]
    fn feil_naar_gjeldende_mangler() {
        let result = sjekk_status(None, None);
        assert!(matches!(result, Err(SjekkFeil::ManglerGjeldende)));
    }
}
