#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[repr(i16)]
pub enum RegelId {
    HeleTraceSignert = 1,
    IngenPeriodeUtenStartet = 2,
    MeldingHarTraceId = 3,
    StartetErSignert = 4,
    StartetGirPeriode = 5,
}

impl RegelId {
    pub const ALLE: [RegelId; 5] = [
        Self::HeleTraceSignert,
        Self::IngenPeriodeUtenStartet,
        Self::MeldingHarTraceId,
        Self::StartetErSignert,
        Self::StartetGirPeriode,
    ];

    pub fn fra_i16(verdi: i16) -> Option<Self> {
        Self::ALLE.into_iter().find(|r| *r as i16 == verdi)
    }
}
