use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodeStatistics {
    pub totalt: i64,
    pub er_aktiv: i64,
    pub er_avsluttet: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedighetStatistics {
    pub er_null: i64,
    pub er_ikke_null: i64,
    pub over_0030_dager: i64,
    pub over_0060_dager: i64,
    pub over_0090_dager: i64,
    pub over_0180_dager: i64,
    pub over_0365_dager: i64,
    pub over_0730_dager: i64,
    pub over_1095_dager: i64,
}
