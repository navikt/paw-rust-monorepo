use serde::{Deserialize, Serialize};

const ENTRA_ID: &str = "entra_id";
const TOKENX: &str = "tokenx";

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

#[derive(Debug, Serialize, Deserialize)]
pub struct OBOTokenRequest {
    identity_provider: &'static str,
    user_token: String,
    target: String,
}

impl OBOTokenRequest {
    pub fn new(user_token: String, target: String) -> Self {
        Self {
            identity_provider: TOKENX,
            user_token,
            target,
        }
    }
}
