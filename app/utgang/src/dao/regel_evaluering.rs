use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::EnumString;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Status {
    Godkjent,
    Avvist,
    KreverManuellVurdering,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegelEvaluering {
    pub tidspunkt: DateTime<Utc>,
    pub regelsett_versjon: String,
    pub status: Status,
    pub regel_ider: Vec<String>,
}

impl PartialEq for RegelEvaluering {
    fn eq(&self, other: &Self) -> bool {
        self.regelsett_versjon == other.regelsett_versjon
            && self.status == other.status
            && self.regel_ider.len() == other.regel_ider.len()
            && self
                .regel_ider
                .iter()
                .all(|id| other.regel_ider.contains(id))
    }
}

impl Eq for RegelEvaluering {}
