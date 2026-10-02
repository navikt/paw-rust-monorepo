use chrono::{DateTime, TimeDelta, Utc};
use interne_hendelser::vo::Bruker;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{duration, instant};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeveringsfristUtloept {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    /// Unik id for personen, generert av kafka-key-generator.
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub bekreftelse_id: Uuid,
    #[serde(with = "instant")]
    pub leveringsfrist: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EksternGracePeriodeUtloept {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub paa_vegne_av_namespace: String,
    pub paa_vegne_av_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterGracePeriodeUtloept {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub bekreftelse_id: Uuid,
    #[serde(default)]
    pub kilde: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BekreftelseTilgjengelig {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub bekreftelse_id: Uuid,
    #[serde(with = "instant")]
    pub gjelder_fra: DateTime<Utc>,
    #[serde(with = "instant")]
    pub gjelder_til: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BekreftelseMeldingMottatt {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub bekreftelse_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodeAvsluttet {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterGracePeriodeGjenstaaendeTid {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub bekreftelse_id: Uuid,
    #[serde(with = "duration")]
    pub gjenstaande_tid: TimeDelta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaOmAaAvsluttePeriode {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub utfoert_av: Bruker,
    #[serde(default)]
    pub kilde: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BekreftelsePaaVegneAvStartet {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterGracePeriodeUtloeptEtterEksternInnsamling {
    pub hendelse_id: Uuid,
    pub periode_id: Uuid,
    pub arbeidssoeker_id: i64,
    #[serde(with = "instant")]
    pub hendelse_tidspunkt: DateTime<Utc>,
    #[serde(default)]
    pub kilde: String,
}
