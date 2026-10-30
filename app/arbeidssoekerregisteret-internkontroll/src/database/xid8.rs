use std::fmt;

use sqlx::{
    Decode, Encode, Postgres, Type,
    encode::IsNull,
    error::BoxDynError,
    postgres::{PgArgumentBuffer, PgTypeInfo, PgValueFormat, PgValueRef},
};

/// A PostgreSQL `xid8`: a 64-bit transaction id that never wraps around.
/// Bound and read directly as `xid8`, so queries need no casts.
/// The binary wire format is an 8-byte big-endian unsigned integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Xid8(u64);

impl Xid8 {
    /// Below every real transaction id: PostgreSQL never assigns 0.
    pub const START: Xid8 = Xid8(0);

    #[allow(unused)]
    pub const fn new(verdi: u64) -> Self {
        Self(verdi)
    }
}

impl fmt::Display for Xid8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Type<Postgres> for Xid8 {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("xid8")
    }
}

impl Encode<'_, Postgres> for Xid8 {
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        buf.extend_from_slice(&self.0.to_be_bytes());
        Ok(IsNull::No)
    }
}

impl Decode<'_, Postgres> for Xid8 {
    fn decode(value: PgValueRef<'_>) -> Result<Self, BoxDynError> {
        let verdi = match value.format() {
            PgValueFormat::Binary => u64::from_be_bytes(value.as_bytes()?.try_into()?),
            PgValueFormat::Text => value.as_str()?.parse()?,
        };
        Ok(Self(verdi))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;

    #[tokio::test]
    async fn round_trip_mot_postgres_uten_cast() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        for verdi in [0, 1, 4_242, i64::MAX as u64, u64::MAX] {
            let xid = Xid8::new(verdi);
            let tilbake: Xid8 = sqlx::query_scalar("SELECT $1")
                .bind(xid)
                .fetch_one(pool)
                .await
                .unwrap();
            assert_eq!(tilbake, xid);
            let tekst: String = sqlx::query_scalar("SELECT $1::text")
                .bind(xid)
                .fetch_one(pool)
                .await
                .unwrap();
            assert_eq!(tekst, verdi.to_string());
        }
    }

    #[tokio::test]
    async fn leser_xid8_fra_postgres() {
        let guard = setup_postgres_container().await.unwrap();
        let pool = &guard.pg_pool;
        let xid: Xid8 = sqlx::query_scalar("SELECT '18446744073709551615'::xid8")
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(xid, Xid8::new(u64::MAX));
        let mindre: bool = sqlx::query_scalar("SELECT $1 < $2")
            .bind(Xid8::new(5))
            .bind(Xid8::new(6))
            .fetch_one(pool)
            .await
            .unwrap();
        assert!(mindre);
    }
}
