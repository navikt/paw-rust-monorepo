use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InnflyttingTilNorge {
    pub fraflyttingsland: Option<String>,
    pub fraflyttingssted_i_utlandet: Option<String>,
}
