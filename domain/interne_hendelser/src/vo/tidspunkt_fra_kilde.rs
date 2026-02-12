use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::AvviksType;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TidspunktFraKilde {
    pub tidspunkt: DateTime<Utc>,
    pub avviks_type: AvviksType,
}
