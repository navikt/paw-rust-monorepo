use serde::{Deserialize, Serialize};

use super::JobbsituasjonMedDetaljer;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Jobbsituasjon {
    pub beskrivelser: Vec<JobbsituasjonMedDetaljer>,
}
