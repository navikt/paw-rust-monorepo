use super::{TeamKometTiltakskode, TeamTiltakTiltakskode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaTiltakstype {
    pub tiltakskode: String,
    pub navn: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamKometTiltakstype {
    pub tiltakskode: TeamKometTiltakskode,
    pub navn: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTiltakTiltakstype {
    pub tiltakskode: TeamTiltakTiltakskode,
    pub navn: String,
}
