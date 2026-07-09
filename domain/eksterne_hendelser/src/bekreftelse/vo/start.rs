use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Start {
    #[serde(rename = "intervalMS")]
    pub interval_ms: i64,
    #[serde(rename = "graceMS")]
    pub grace_ms: i64,
}
