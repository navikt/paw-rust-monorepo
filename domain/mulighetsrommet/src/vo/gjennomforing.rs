use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gjennomforing {
    pub id: String,
    pub navn: Option<String>,
    pub deltidsprosent: Option<f64>,
}
