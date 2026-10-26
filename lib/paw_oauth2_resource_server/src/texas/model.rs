use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectRequest {
    identity_provider: String,
    token: String,
}

impl IntrospectRequest {
    pub fn new(identity_provider: impl Into<String>, token: String) -> Self {
        Self {
            identity_provider: identity_provider.into(),
            token,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectResponse {
    pub active: bool,
    pub error: Option<String>,
}
