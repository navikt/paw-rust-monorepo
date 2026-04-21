use super::regel::Regel;
use super::regel_id::RegelId;
use super::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};
use anyhow::Result;
use interne_hendelser::vo::Opplysning;

pub struct Regelsett {
    pub regler: Vec<Regel>,
    pub standard_regel: Regel,
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
    pub fn evaluer(
        &self,
        opplysninger: &[Opplysning],
    ) -> Result<Vec<GrunnlagForGodkjenning>, Vec<Problem>> {
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
                return Err(vec![problemer.swap_remove(idx)]);
            }
            let skal_avvises = problemer.remove(idx);
            problemer.insert(0, skal_avvises);
            return Err(problemer);
        }

        if !godkjenninger.is_empty() {
            return Ok(godkjenninger);
        }

        if !problemer.is_empty() {
            return Err(problemer);
        }

        self.standard_regel
            .ved_treff(opplysninger.to_vec())
            .map(|g| vec![g])
            .map_err(|p| vec![p])
    }

    pub fn evaluer_liste<'a, T>(
        &self,
        opplysninger: &'a [(T, Vec<Opplysning>)],
    ) -> Vec<(&'a T, Result<Vec<GrunnlagForGodkjenning>, Vec<Problem>>)> {
        opplysninger
            .iter()
            .map(|(a, opplysning)| (a, self.evaluer(opplysning)))
            .collect()
    }
}
