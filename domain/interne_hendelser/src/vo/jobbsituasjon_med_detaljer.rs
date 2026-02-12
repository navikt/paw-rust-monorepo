use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use super::JobbsituasjonBeskrivelse;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobbsituasjonMedDetaljer {
    pub beskrivelse: JobbsituasjonBeskrivelse,
    pub detaljer: HashMap<String, String>,
}
