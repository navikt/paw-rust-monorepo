use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use uuid::Uuid;

#[derive(Serialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatePoliciesRequest {
    pub requests: Vec<PolicyRequest>,
}

impl EvaluatePoliciesRequest {
    pub fn new(requests: Vec<PolicyRequest>) -> Self {
        Self { requests }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PolicyRequest {
    pub request_id: Uuid,
    #[serde(flatten)]
    pub policy: Policy,
}

impl PolicyRequest {
    pub fn new(policy: &Policy) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            policy: policy.clone(),
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(
    tag = "policyId",
    content = "policyInput",
    rename_all = "SCREAMING_SNAKE_CASE"
)]
pub enum Policy {
    NavAnsattNavIdentLesetilgangTilEksternBrukerV1(NavAnsattTilgangTilEksternBrukerPolicyInput),
    NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1(NavAnsattTilgangTilEksternBrukerPolicyInput),
    NavAnsattNavIdentTilgangTilNavEnhetV1(NavAnsattTilgangTilNavEnhetPolicyInput),
}

impl Policy {
    pub fn nav_ansatt_lesetilgang_til_ekstern_bruker_v1(
        nav_ident: String,
        norsk_ident: String,
    ) -> Self {
        Policy::NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1(
            NavAnsattTilgangTilEksternBrukerPolicyInput::new(nav_ident, norsk_ident),
        )
    }
    pub fn nav_ansatt_skrivetilgang_til_ekstern_bruker_v1(
        nav_ident: String,
        norsk_ident: String,
    ) -> Self {
        Policy::NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1(
            NavAnsattTilgangTilEksternBrukerPolicyInput::new(nav_ident, norsk_ident),
        )
    }
    pub fn nav_ansatt_tilgang_til_nav_enhet_v1(nav_ident: String, nav_enhet_id: String) -> Self {
        Policy::NavAnsattNavIdentTilgangTilNavEnhetV1(NavAnsattTilgangTilNavEnhetPolicyInput::new(
            nav_ident,
            nav_enhet_id,
        ))
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattTilgangTilEksternBrukerPolicyInput {
    pub nav_ident: String,
    pub norsk_ident: String,
}

impl NavAnsattTilgangTilEksternBrukerPolicyInput {
    pub fn new(nav_ident: String, norsk_ident: String) -> Self {
        Self {
            nav_ident,
            norsk_ident,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattTilgangTilNavEnhetPolicyInput {
    pub nav_ident: String,
    pub nav_enhet_id: String,
}

impl NavAnsattTilgangTilNavEnhetPolicyInput {
    pub fn new(nav_ident: String, nav_enhet_id: String) -> Self {
        Self {
            nav_ident,
            nav_enhet_id,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatePoliciesResponse {
    pub results: Vec<PolicyEvaluationResult>,
}

impl EvaluatePoliciesResponse {
    pub fn is_permitted(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.decision.decision_type == DecisionType::Permit)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEvaluationResult {
    pub request_id: Uuid,
    pub decision: PolicyDecision,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PolicyDecision {
    #[serde(rename = "type")]
    pub decision_type: DecisionType,
    pub message: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DecisionType {
    Permit,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyId {
    NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1,
    NavAnsattNavIdentLesetilgangTilEksternBrukerV1,
    NavAnsattNavIdentTilgangTilNavEnhetV1,
}

pub fn enum_type_not_found(type_: &str) -> EnumTypeParseError {
    EnumTypeParseError::UkjentType(type_.to_string())
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum EnumTypeParseError {
    #[error("Ukjent enum: {0}")]
    UkjentType(String),
}
