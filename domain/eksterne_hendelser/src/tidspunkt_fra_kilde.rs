use crate::avvikstype::AvviksType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampMilliSeconds};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TidspunktFraKilde {
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub tidspunkt: DateTime<Utc>,
    pub avviks_type: AvviksType,
}
