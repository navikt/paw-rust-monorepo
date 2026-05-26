use super::regel::Regel;
use super::regel_id::RegelId;
use interne_hendelser::vo::Opplysning;
use serde::{Deserialize, Serialize};

pub struct Regelsett {
    pub regler: Vec<Regel>,
    pub standard_regel: Regel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalueringsResultat {
    GrunnlagForGodkjenning { regel_ider: Vec<RegelId> },
    Avvist { regel_ider: Vec<RegelId> },
    KreverManuellVurdering { regel_ider: Vec<RegelId> },
}

#[derive(Default)]
pub struct Eval {
    manuell_vurdering: Vec<RegelId>,
    godkjent: Vec<RegelId>,
    avvist: Vec<RegelId>,
}

impl EvalueringsResultat {
    pub fn is_grunnlag_for_godkjenning(&self) -> bool {
        matches!(self, Self::GrunnlagForGodkjenning { .. })
    }

    pub fn is_avvist(&self) -> bool {
        matches!(self, Self::Avvist { .. })
    }

    pub fn is_krever_manuell_vurdering(&self) -> bool {
        matches!(self, Self::KreverManuellVurdering { .. })
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::GrunnlagForGodkjenning { .. } => "GODKJENT",
            Self::Avvist { .. } => "AVVIST",
            Self::KreverManuellVurdering { .. } => "KREVER_MANUELL_VURDERING",
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
        let eval = self.regler.iter().filter(|r| r.evaluer(opplysninger)).fold(
            Eval::default(),
            |mut eval, regel| {
                match regel.ved_treff() {
                    EvalueringsResultat::GrunnlagForGodkjenning { regel_ider } => {
                        eval.godkjent.extend(regel_ider)
                    }
                    EvalueringsResultat::Avvist { regel_ider } => eval.avvist.extend(regel_ider),
                    EvalueringsResultat::KreverManuellVurdering { regel_ider } => {
                        eval.manuell_vurdering.extend(regel_ider)
                    }
                };
                eval
            },
        );
        match eval {
            eval if eval.avvist.contains(&RegelId::IkkeFunnet) => EvalueringsResultat::Avvist {
                regel_ider: vec![RegelId::IkkeFunnet],
            },
            eval if !eval.avvist.is_empty() => EvalueringsResultat::Avvist {
                regel_ider: eval
                    .avvist
                    .into_iter()
                    .chain(eval.manuell_vurdering)
                    .collect(),
            },
            eval if !eval.manuell_vurdering.is_empty() => {
                EvalueringsResultat::KreverManuellVurdering {
                    regel_ider: eval.manuell_vurdering,
                }
            }
            eval if !eval.godkjent.is_empty() => EvalueringsResultat::GrunnlagForGodkjenning {
                regel_ider: eval.godkjent,
            },
            _ => self.standard_regel.ved_treff(),
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
