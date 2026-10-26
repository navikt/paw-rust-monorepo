use sqlx::prelude::FromRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[repr(i16)]
pub enum RegelId {
    HeleTraceSignert = 1,
    IngenPeriodeUtenStartet = 2,
}
