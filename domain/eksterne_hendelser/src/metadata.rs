use crate::bruker::Bruker;
use crate::tidspunkt_fra_kilde::TidspunktFraKilde;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampMilliSeconds};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Timestamp of the change
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub tidspunkt: DateTime<Utc>,
    /// Who performed the change
    pub utfoert_av: Bruker,
    /// Name of the system that performed the change
    pub kilde: String,
    /// Reason for the change
    pub aarsak: String,
    /// Time deviation from source, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tidspunkt_fra_kilde: Option<TidspunktFraKilde>,
}
