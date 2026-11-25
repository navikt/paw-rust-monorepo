#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[repr(i16)]
pub enum RegelId {
    HeleTraceSignert = 1,
    IngenPeriodeUtenStartet = 2,
    StartetErSignert = 3,
    StartetGirPeriode = 4,
    PeriodeStartetLikStartet = 5,
}

impl RegelId {
    pub const ALLE: [RegelId; 5] = [
        Self::HeleTraceSignert,
        Self::IngenPeriodeUtenStartet,
        Self::StartetErSignert,
        Self::StartetGirPeriode,
        Self::PeriodeStartetLikStartet,
    ];

    pub fn fra_i16(verdi: i16) -> Option<Self> {
        Self::ALLE.into_iter().find(|r| *r as i16 == verdi)
    }
}
