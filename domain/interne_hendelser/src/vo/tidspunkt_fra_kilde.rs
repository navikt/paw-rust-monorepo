use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{unix_timestamp, AvviksType};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TidspunktFraKilde {
    #[serde(with = "unix_timestamp")]
    pub tidspunkt: DateTime<Utc>,
    pub avviks_type: AvviksType,
}
