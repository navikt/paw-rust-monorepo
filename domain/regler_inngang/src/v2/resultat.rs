use interne_hendelser::vo::Opplysning;
use super::regel_id::RegelId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    SkalAvvises,
    MuligGrunnlagForAvvisning,
}

#[derive(Debug, Clone)]
pub struct Problem {
    pub regel_id: RegelId,
    pub opplysninger: Vec<Opplysning>,
    pub kind: ProblemKind,
}

#[derive(Debug, Clone)]
pub struct GrunnlagForGodkjenning {
    pub regel_id: RegelId,
    pub opplysninger: Vec<Opplysning>,
}
