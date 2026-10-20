use poao_tilgang_client::model::{
    EvaluatePoliciesRequest, NavAnsattTilgangTilEksternBrukerPolicyInput,
    NavAnsattTilgangTilNavEnhetPolicyInput, Policy, PolicyId, PolicyRequest,
};
use serde_json::{Value, json};
use uuid::Uuid;

fn assert_policy_json(policy: Policy, policy_id: PolicyId, expected_json: &str) {
    let serialized = serde_json::to_string(&policy).unwrap();
    assert_eq!(serialized, expected_json);

    let policy_json: Value = serde_json::from_str(&serialized).unwrap();
    assert_eq!(
        policy_json["policyId"],
        serde_json::to_value(policy_id).unwrap()
    );
}

#[test]
fn serializes_external_user_write_policy() {
    assert_policy_json(
        Policy::NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1(
            NavAnsattTilgangTilEksternBrukerPolicyInput {
                nav_ident: "Z123456".to_string(),
                norsk_ident: "01017012345".to_string(),
            },
        ),
        PolicyId::NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1,
        r#"{"policyId":"NAV_ANSATT_NAV_IDENT_SKRIVETILGANG_TIL_EKSTERN_BRUKER_V1","policyInput":{"navIdent":"Z123456","norskIdent":"01017012345"}}"#,
    );
}

#[test]
fn serializes_external_user_read_policy() {
    assert_policy_json(
        Policy::NavAnsattNavIdentLesetilgangTilEksternBrukerV1(
            NavAnsattTilgangTilEksternBrukerPolicyInput {
                nav_ident: "Z234567".to_string(),
                norsk_ident: "02017012345".to_string(),
            },
        ),
        PolicyId::NavAnsattNavIdentLesetilgangTilEksternBrukerV1,
        r#"{"policyId":"NAV_ANSATT_NAV_IDENT_LESETILGANG_TIL_EKSTERN_BRUKER_V1","policyInput":{"navIdent":"Z234567","norskIdent":"02017012345"}}"#,
    );
}

#[test]
fn serializes_nav_unit_access_policy() {
    assert_policy_json(
        Policy::NavAnsattNavIdentTilgangTilNavEnhetV1(NavAnsattTilgangTilNavEnhetPolicyInput {
            nav_ident: "Z345678".to_string(),
            nav_enhet_id: "03017012345".to_string(),
        }),
        PolicyId::NavAnsattNavIdentTilgangTilNavEnhetV1,
        r#"{"policyId":"NAV_ANSATT_NAV_IDENT_TILGANG_TIL_NAV_ENHET_V1","policyInput":{"navIdent":"Z345678","navEnhetId":"03017012345"}}"#,
    );
}

#[test]
fn serializes_heterogeneous_and_empty_batches() {
    let request = EvaluatePoliciesRequest {
        requests: vec![
            PolicyRequest {
                request_id: Uuid::from_u128(1),
                policy: Policy::NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1(
                    NavAnsattTilgangTilEksternBrukerPolicyInput {
                        nav_ident: "Z123456".to_string(),
                        norsk_ident: "01017012345".to_string(),
                    },
                ),
            },
            PolicyRequest {
                request_id: Uuid::from_u128(2),
                policy: Policy::NavAnsattNavIdentTilgangTilNavEnhetV1(
                    NavAnsattTilgangTilNavEnhetPolicyInput {
                        nav_ident: "Z345678".to_string(),
                        nav_enhet_id: "03017012345".to_string(),
                    },
                ),
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(request).unwrap(),
        json!({
            "requests": [
                {
                    "requestId": "00000000-0000-0000-0000-000000000001",
                    "policyId": "NAV_ANSATT_NAV_IDENT_SKRIVETILGANG_TIL_EKSTERN_BRUKER_V1",
                    "policyInput": {
                        "navIdent": "Z123456",
                        "norskIdent": "01017012345"
                    }
                },
                {
                    "requestId": "00000000-0000-0000-0000-000000000002",
                    "policyId": "NAV_ANSATT_NAV_IDENT_TILGANG_TIL_NAV_ENHET_V1",
                    "policyInput": {
                        "navIdent": "Z345678",
                        "navEnhetId": "03017012345"
                    }
                }
            ]
        })
    );
    assert_eq!(
        serde_json::to_value(EvaluatePoliciesRequest::new(Vec::new())).unwrap(),
        json!({"requests": []})
    );
}
