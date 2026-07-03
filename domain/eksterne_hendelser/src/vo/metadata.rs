use crate::vo::bruker::Bruker;
use crate::vo::tidspunkt_fra_kilde::TidspunktFraKilde;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampMilliSeconds};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub tidspunkt: DateTime<Utc>,
    pub utfoert_av: Bruker,
    pub kilde: String,
    pub aarsak: String,
    pub tidspunkt_fra_kilde: Option<TidspunktFraKilde>,
}
