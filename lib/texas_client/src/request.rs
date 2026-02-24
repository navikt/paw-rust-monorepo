use serde::{Deserialize, Serialize};

const ENTRA_ID: &str = "entra_id";

#[derive(Debug, Serialize, Deserialize)]
pub struct M2MTokenRequest {
    identity_provider: &'static str,
    target: String,
}

impl M2MTokenRequest {
    pub fn new(target: String) -> Self {
        Self {
            identity_provider: ENTRA_ID,
            target,
        }
    }
}
