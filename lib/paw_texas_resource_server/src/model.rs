use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectRequest {
    identity_provider: &'static str,
    token: String,
}

impl IntrospectRequest {
    pub fn new(identity_provider: &'static str, token: String) -> Self {
        Self {
            identity_provider,
            token,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IntrospectResponse {
    pub active: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "snake_case",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
pub enum IdentityProvider {
    #[strum(serialize = "tokenx")]
    TokenX,
    EntraId,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, EnumString, AsRefStr)]
#[strum(
    serialize_all = "snake_case",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
pub enum OAuthErrorCode {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnauthorizedClient,
    UnsupportedGrantType,
    InvalidScope,
    ServerError,
    #[serde(other)]
    #[default]
    UkjentVerdi,
}

pub fn enum_type_not_found(type_: &str) -> EnumTypeParseError {
    EnumTypeParseError::UkjentType(type_.to_string())
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EnumTypeParseError {
    #[error("Ukjent enum: {0}")]
    UkjentType(String),
}
