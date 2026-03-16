use super::regel_id::RegelId;
use interne_hendelser::vo::Opplysning;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    SkalAvvises,
    MuligGrunnlagForAvvisning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub regel_id: RegelId,
    pub opplysninger: Vec<Opplysning>,
    pub kind: ProblemKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrunnlagForGodkjenning {
    pub regel_id: RegelId,
    pub opplysninger: Vec<Opplysning>,
}
