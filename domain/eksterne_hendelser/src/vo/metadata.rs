use crate::vo::bruker::Bruker;
use crate::vo::tidspunkt_fra_kilde::TidspunktFraKilde;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampMilliSeconds, serde_as};

pub trait Metadata {
    fn tidspunkt(&self) -> &DateTime<Utc>;
    fn utfoert_av(&self) -> &Bruker;
    fn kilde(&self) -> &str;
    fn aarsak(&self) -> &str;
    fn tidspunkt_fra_kilde(&self) -> Option<&TidspunktFraKilde>;
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MainMetadata {
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub tidspunkt: DateTime<Utc>,
    pub utfoert_av: Bruker,
    pub kilde: String,
    pub aarsak: String,
    pub tidspunkt_fra_kilde: Option<TidspunktFraKilde>,
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BekreftelseMetadata {
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub tidspunkt: DateTime<Utc>,
    pub utfoert_av: Bruker,
    pub kilde: String,
    pub aarsak: String,
}

impl Metadata for MainMetadata {
    fn tidspunkt(&self) -> &DateTime<Utc> {
        &self.tidspunkt
    }

    fn utfoert_av(&self) -> &Bruker {
        &self.utfoert_av
    }

    fn kilde(&self) -> &str {
        &self.kilde
    }

    fn aarsak(&self) -> &str {
        &self.aarsak
    }

    fn tidspunkt_fra_kilde(&self) -> Option<&TidspunktFraKilde> {
        self.tidspunkt_fra_kilde.as_ref()
    }
}

impl Metadata for BekreftelseMetadata {
    fn tidspunkt(&self) -> &DateTime<Utc> {
        &self.tidspunkt
    }

    fn utfoert_av(&self) -> &Bruker {
        &self.utfoert_av
    }

    fn kilde(&self) -> &str {
        &self.kilde
    }

    fn aarsak(&self) -> &str {
        &self.aarsak
    }

    fn tidspunkt_fra_kilde(&self) -> Option<&TidspunktFraKilde> {
        None
    }
}
