#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[repr(i16)]
pub enum RegelId {
    HeleTraceSignert = 1,
    IngenPeriodeUtenStartet = 2,
    MeldingHarTraceId = 3,
}

impl RegelId {
    pub const ALLE: [RegelId; 3] = [
        Self::HeleTraceSignert,
        Self::IngenPeriodeUtenStartet,
        Self::MeldingHarTraceId,
    ];

    pub fn fra_i16(verdi: i16) -> Option<Self> {
        Self::ALLE.into_iter().find(|r| *r as i16 == verdi)
    }
}
