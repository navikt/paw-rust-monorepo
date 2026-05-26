use super::regel::Regel;
use super::regel_id::RegelId;
use super::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};
use interne_hendelser::vo::Opplysning;
use serde::{Deserialize, Serialize};

pub struct Regelsett {
    pub regler: Vec<Regel>,
    pub standard_regel: Regel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalueringsResultat {
    Godkjent { grunnlag: Vec<GrunnlagForGodkjenning> },
    Avvist { problemer: Vec<Problem> },
}

impl EvalueringsResultat {
    pub fn is_godkjent(&self) -> bool {
        matches!(self, Self::Godkjent { .. })
    }

    pub fn is_avvist(&self) -> bool {
        matches!(self, Self::Avvist { .. })
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::Godkjent { .. } => "GODKJENT",
            Self::Avvist { .. } => "AVVIST",
        }
    }
}

impl Regelsett {
    /// Evaluates the rule set against the given opplysninger.
    ///
    /// Priority:
    /// 1. `SkalAvvises` for `IkkeFunnet` → return only that problem.
    /// 2. Any `SkalAvvises` → return it first, then remaining problems.
    /// 3. Any `GrunnlagForGodkjenning` → return all matching.
    /// 4. Any problems → return them all.
    /// 5. No rules matched → apply `standard_regel`.
    pub fn evaluer(&self, opplysninger: &[Opplysning]) -> EvalueringsResultat {
        let mut problemer: Vec<Problem> = Vec::new();
        let mut godkjenninger: Vec<GrunnlagForGodkjenning> = Vec::new();

        for regel in self.regler.iter().filter(|r| r.evaluer(opplysninger)) {
            match regel.ved_treff(opplysninger.to_vec()) {
                Ok(g) => godkjenninger.push(g),
                Err(p) => problemer.push(p),
            }
        }

        if let Some(idx) = problemer
            .iter()
            .position(|p| p.kind == ProblemKind::SkalAvvises)
        {
            if problemer[idx].regel_id == RegelId::IkkeFunnet {
                return EvalueringsResultat::Avvist {
                    problemer: vec![problemer.swap_remove(idx)],
                };
            }
            let skal_avvises = problemer.remove(idx);
            problemer.insert(0, skal_avvises);
            return EvalueringsResultat::Avvist { problemer };
        }

        if !godkjenninger.is_empty() {
            return EvalueringsResultat::Godkjent {
                grunnlag: godkjenninger,
            };
        }

        if !problemer.is_empty() {
            return EvalueringsResultat::Avvist { problemer };
        }

        match self.standard_regel.ved_treff(opplysninger.to_vec()) {
            Ok(g) => EvalueringsResultat::Godkjent { grunnlag: vec![g] },
            Err(p) => EvalueringsResultat::Avvist {
                problemer: vec![p],
            },
        }
    }

    pub fn evaluer_liste<'a, T>(
        &self,
        opplysninger: &'a [(T, Vec<Opplysning>)],
    ) -> Vec<(&'a T, EvalueringsResultat)> {
        opplysninger
            .iter()
            .map(|(a, opplysning)| (a, self.evaluer(opplysning)))
            .collect()
    }
}
