use crate::identiteter::identitet::Identitet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "hendelseType")]
pub enum IdentitetHendelse {
    #[serde(rename = "identitet.v1.identiteter_endret")]
    IdentiteterEndret(IdentiteterEndretHendelse),
    #[serde(rename = "identitet.v1.identiteter_merget")]
    IdentiteterMerget(IdentiteterMergetHendelse),
    #[serde(rename = "identitet.v1.identiteter_splittet")]
    IdentiteterSplittet(IdentiteterSplittetHendelse),
    #[serde(rename = "identitet.v1.identiteter_slettet")]
    IdentiteterSlettet(IdentiteterSlettetHendelse),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentiteterEndretHendelse {
    pub hendelse_id: Uuid,
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub identiteter: Vec<Identitet>,
    #[serde(default)]
    pub tidligere_identiteter: Vec<Identitet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentiteterMergetHendelse {
    pub hendelse_id: Uuid,
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub identiteter: Vec<Identitet>,
    #[serde(default)]
    pub tidligere_identiteter: Vec<Identitet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentiteterSplittetHendelse {
    pub hendelse_id: Uuid,
    pub hendelse_tidspunkt: DateTime<Utc>,
    pub identiteter: Vec<Identitet>,
    #[serde(default)]
    pub tidligere_identiteter: Vec<Identitet>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentiteterSlettetHendelse {
    pub hendelse_id: Uuid,
    pub hendelse_tidspunkt: DateTime<Utc>,
    #[serde(default)]
    pub tidligere_identiteter: Vec<Identitet>,
}
