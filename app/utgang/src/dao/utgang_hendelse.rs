use std::collections::HashSet;
use std::str::FromStr;

use chrono::NaiveDateTime;
use interne_hendelser::vo::{BrukerType, Opplysning};
use sqlx::Row;
use uuid::Uuid;

use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use crate::domain::utgang_hendelse_type::UtgangHendelseType;
use interne_hendelser::vo::Opplysninger;

pub struct Input;
pub struct Output;

pub struct InternUtgangHendelse<A> {
    primary_key: Option<i64>,
    hendelsetype: UtgangHendelseType,
    periode_id: ArbeidssoekerperiodeId,
    timestamp: chrono::DateTime<chrono::Utc>,
    brukertype: BrukerType,
    opplysninger: Option<Opplysninger>,
    phantom: std::marker::PhantomData<A>,
}

impl InternUtgangHendelse<Input> {
    pub fn new(
        hendelsetype: UtgangHendelseType,
        periode_id: ArbeidssoekerperiodeId,
        timestamp: chrono::DateTime<chrono::Utc>,
        brukertype: BrukerType,
        opplysninger: Option<Opplysninger>,
    ) -> Self {
        Self {
            primary_key: None,
            hendelsetype,
            periode_id,
            timestamp,
            brukertype,
            opplysninger,
            phantom: std::marker::PhantomData,
        }
    }
}

impl InternUtgangHendelse<Output> {
    pub(crate) fn from_db_row(
        primary_key: i64,
        hendelsetype: UtgangHendelseType,
        periode_id: ArbeidssoekerperiodeId,
        timestamp: chrono::DateTime<chrono::Utc>,
        brukertype: BrukerType,
        opplysninger: Option<Opplysninger>,
    ) -> Self {
        Self {
            primary_key: Some(primary_key),
            hendelsetype,
            periode_id,
            timestamp,
            brukertype,
            opplysninger,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn primary_key(&self) -> i64 {
        self.primary_key
            .expect("Output-hendelse må ha en primary key")
    }
}

impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for InternUtgangHendelse<Output> {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        let id: i64 = row.try_get("id")?;
        let timestamp: NaiveDateTime = row.try_get("timestamp")?;
        let type_str: String = row.try_get("type")?;
        let periode_id: Uuid = row.try_get("periode_id")?;
        let brukertype_str: String = row.try_get("brukertype")?;
        let opplysninger_str: Option<Vec<String>> = row.try_get("opplysninger")?;

        let hendelsetype =
            UtgangHendelseType::from_str(&type_str).map_err(|e| sqlx::Error::ColumnDecode {
                index: "type".into(),
                source: Box::new(e),
            })?;
        let brukertype =
            BrukerType::from_str(&brukertype_str).map_err(|e| sqlx::Error::ColumnDecode {
                index: "brukertype".into(),
                source: Box::new(e),
            })?;
        let opplysninger = opplysninger_str
            .map(|strings| {
                strings
                    .into_iter()
                    .filter_map(|s| Opplysning::from_str(&s).ok())
                    .collect::<HashSet<_>>()
            })
            .map(Opplysninger);

        Ok(Self::from_db_row(
            id,
            hendelsetype,
            ArbeidssoekerperiodeId::from(periode_id),
            timestamp.and_utc(),
            brukertype,
            opplysninger,
        ))
    }
}

impl<A> InternUtgangHendelse<A> {
    pub fn hendelsetype(&self) -> &UtgangHendelseType {
        &self.hendelsetype
    }

    pub fn periode_id(&self) -> &ArbeidssoekerperiodeId {
        &self.periode_id
    }

    pub fn timestamp(&self) -> chrono::DateTime<chrono::Utc> {
        self.timestamp
    }

    pub fn brukertype(&self) -> &BrukerType {
        &self.brukertype
    }

    pub fn opplysninger(&self) -> Option<&Opplysninger> {
        self.opplysninger.as_ref()
    }
}
