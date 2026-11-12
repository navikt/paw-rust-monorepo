use super::{
    ArenaStatus, ArenaTiltakstype, Arrangor, Gjennomforing, TeamKometStatus, TeamKometTiltakstype,
    TeamTiltakStatus, TeamTiltakTiltakstype, TiltakshistorikkOpphav, Virksomhet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Deltakelse {
    ArenaDeltakelse(ArenaDeltakelse),
    TeamKometDeltakelse(TeamKometDeltakelse),
    TeamTiltakAvtale(TeamTiltakAvtale),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArenaDeltakelse {
    pub norsk_ident: String,
    pub start_dato: Option<String>,
    pub slutt_dato: Option<String>,
    pub opprettet_tidspunkt: String,
    pub oppdatert_tidspunkt: String,
    pub id: String,
    pub tittel: String,
    pub arena_id: i64,
    pub status: ArenaStatus,
    pub tiltakstype: ArenaTiltakstype,
    pub gjennomforing: Gjennomforing,
    pub arrangor: Arrangor,
    pub deltidsprosent: Option<f64>,
    pub dager_per_uke: Option<f64>,
    pub opphav: Option<TiltakshistorikkOpphav>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamKometDeltakelse {
    pub norsk_ident: String,
    pub start_dato: Option<String>,
    pub slutt_dato: Option<String>,
    pub opprettet_tidspunkt: String,
    pub oppdatert_tidspunkt: String,
    pub id: String,
    pub tittel: String,
    pub status: TeamKometStatus,
    pub tiltakstype: TeamKometTiltakstype,
    pub gjennomforing: Gjennomforing,
    pub arrangor: Arrangor,
    pub deltidsprosent: Option<f64>,
    pub dager_per_uke: Option<f64>,
    pub opphav: Option<TiltakshistorikkOpphav>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamTiltakAvtale {
    pub norsk_ident: String,
    pub start_dato: Option<String>,
    pub slutt_dato: Option<String>,
    pub opprettet_tidspunkt: String,
    pub oppdatert_tidspunkt: String,
    pub id: String,
    pub tittel: String,
    pub tiltakstype: TeamTiltakTiltakstype,
    pub status: TeamTiltakStatus,
    pub stillingsprosent: Option<f64>,
    pub dager_per_uke: Option<f64>,
    pub arbeidsgiver: Virksomhet,
    pub opphav: Option<TiltakshistorikkOpphav>,
}
