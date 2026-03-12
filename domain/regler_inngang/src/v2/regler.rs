use interne_hendelser::vo::Opplysning;
use super::regel::Regel;
use super::regel_id::RegelId;
use super::resultat::{GrunnlagForGodkjenning, Problem, ProblemKind};

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
    /// 3. Any `GrunnlagForGodkjenning` → return the first one.
    /// 4. Any problems → return them all.
    /// 5. No rules matched → apply `standard_regel`.
    pub fn evaluer(
        &self,
        opplysninger: &[Opplysning],
    ) -> Result<GrunnlagForGodkjenning, Vec<Problem>> {
        let mut problemer: Vec<Problem> = Vec::new();
        let mut godkjenning: Option<GrunnlagForGodkjenning> = None;

        for regel in self.regler.iter().filter(|r| r.evaluer(opplysninger)) {
            match regel.ved_treff(opplysninger.to_vec()) {
                Ok(g) if godkjenning.is_none() => godkjenning = Some(g),
                Ok(_) => {}
                Err(p) => problemer.push(p),
            }
        }

        if let Some(idx) = problemer.iter().position(|p| p.kind == ProblemKind::SkalAvvises) {
            if problemer[idx].regel_id == RegelId::IkkeFunnet {
                return Err(vec![problemer.swap_remove(idx)]);
            }
            let skal_avvises = problemer.remove(idx);
            problemer.insert(0, skal_avvises);
            return Err(problemer);
        }

        if let Some(g) = godkjenning {
            return Ok(g);
        }

        if !problemer.is_empty() {
            return Err(problemer);
        }

        self.standard_regel
            .ved_treff(opplysninger.to_vec())
            .map_err(|p| vec![p])
    }
}
