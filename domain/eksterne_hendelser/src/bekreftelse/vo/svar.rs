use crate::vo::metadata::BekreftelseMetadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampMilliSeconds, serde_as};

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Svar {
    pub sendt_inn_av: BekreftelseMetadata,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub gjelder_fra: DateTime<Utc>,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub gjelder_til: DateTime<Utc>,
    pub har_jobbet_i_denne_perioden: bool,
    pub vil_fortsette_som_arbeidssoeker: bool,
}
