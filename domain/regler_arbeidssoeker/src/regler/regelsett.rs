pub use crate::regler::evalueringsresultat::Evalueringsresultat;

use super::regel::Regel;
use super::regel_id::RegelId;
use interne_hendelser::vo::Opplysning;

pub struct Regelsett {
    pub regler: Vec<Regel>,
    pub standard_regel: Regel,
}

#[derive(Default)]
pub struct Eval {
    manuell_vurdering: Vec<RegelId>,
    godkjent: Vec<RegelId>,
    avvist: Vec<RegelId>,
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
    pub fn evaluer(&self, opplysninger: &[Opplysning]) -> Evalueringsresultat {
        let eval = self.regler.iter().filter(|r| r.evaluer(opplysninger)).fold(
            Eval::default(),
            |mut eval, regel| {
                match regel.ved_treff() {
                    Evalueringsresultat::Godkjent { regel_ider } => {
                        eval.godkjent.extend(regel_ider)
                    }
                    Evalueringsresultat::Avvist { regel_ider } => eval.avvist.extend(regel_ider),
                    Evalueringsresultat::KreverManuellVurdering { regel_ider } => {
                        eval.manuell_vurdering.extend(regel_ider)
                    }
                };
                eval
            },
        );
        match eval {
            eval if eval.avvist.contains(&RegelId::IkkeFunnet) => Evalueringsresultat::Avvist {
                regel_ider: vec![RegelId::IkkeFunnet],
            },
            eval if !eval.avvist.is_empty() => Evalueringsresultat::Avvist {
                regel_ider: eval
                    .avvist
                    .into_iter()
                    .chain(eval.manuell_vurdering)
                    .collect(),
            },
            eval if !eval.godkjent.is_empty() => Evalueringsresultat::Godkjent {
                regel_ider: eval.godkjent,
            },
            eval if !eval.manuell_vurdering.is_empty() => {
                Evalueringsresultat::KreverManuellVurdering {
                    regel_ider: eval.manuell_vurdering,
                }
            }
            _ => self.standard_regel.ved_treff(),
        }
    }

    pub fn evaluer_liste<'a, T>(
        &self,
        opplysninger: &'a [(T, Vec<Opplysning>)],
    ) -> Vec<(&'a T, Evalueringsresultat)> {
        opplysninger
            .iter()
            .map(|(a, opplysning)| (a, self.evaluer(opplysning)))
            .collect()
    }
}
