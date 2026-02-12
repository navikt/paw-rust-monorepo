use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{Bruker, TidspunktFraKilde};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub tidspunkt: DateTime<Utc>,
    pub utfoert_av: Bruker,
    pub kilde: String,
    pub aarsak: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tidspunkt_fra_kilde: Option<TidspunktFraKilde>,
}
