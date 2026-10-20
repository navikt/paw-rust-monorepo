use poao_tilgang_client::client::PoaoTilgangClient;
use poao_tilgang_client::model::{
    NavAnsattTilgangTilEksternBrukerPolicyInput, NavAnsattTilgangTilNavEnhetPolicyInput, Policy,
};
use serde_json::json;
use std::sync::Arc;
use token_client_stub::TokenClientStub;
use uuid::Uuid;

#[tokio::test]
async fn evaluate_sends_heterogeneous_policy_batch() {
    let mut server = mockito::Server::new_async().await;
    let expected_body = json!({
        "requests": [
            {
                "policyId": "NAV_ANSATT_NAV_IDENT_SKRIVETILGANG_TIL_EKSTERN_BRUKER_V1",
                "policyInput": {
                    "navIdent": "Z123456",
                    "norskIdent": "01017012345"
                }
            },
            {
                "policyId": "NAV_ANSATT_NAV_IDENT_LESETILGANG_TIL_EKSTERN_BRUKER_V1",
                "policyInput": {
                    "navIdent": "Z234567",
                    "norskIdent": "02017012345"
                }
            },
            {
                "policyId": "NAV_ANSATT_NAV_IDENT_TILGANG_TIL_NAV_ENHET_V1",
                "policyInput": {
                    "navIdent": "Z345678",
                    "navEnhetId": "03017012345"
                }
            }
        ]
    });
    let mock = server
        .mock("POST", "/api/v1/policy/evaluate")
        .match_request(move |request| match_mock_body(request, &expected_body))
        .with_status(200)
        .with_body(r#"{"results":true}"#)
        .create_async()
        .await;
    let policies = vec![
        Policy::NavAnsattNavIdentSkrivetilgangTilEksternBrukerV1(
            NavAnsattTilgangTilEksternBrukerPolicyInput {
                nav_ident: "Z123456".to_string(),
                norsk_ident: "01017012345".to_string(),
            },
        ),
        Policy::NavAnsattNavIdentLesetilgangTilEksternBrukerV1(
            NavAnsattTilgangTilEksternBrukerPolicyInput {
                nav_ident: "Z234567".to_string(),
                norsk_ident: "02017012345".to_string(),
            },
        ),
        Policy::NavAnsattNavIdentTilgangTilNavEnhetV1(NavAnsattTilgangTilNavEnhetPolicyInput {
            nav_ident: "Z345678".to_string(),
            nav_enhet_id: "03017012345".to_string(),
        }),
    ];

    let response = client(server.url())
        .evaluate(&policies)
        .await
        .expect("evaluate request should succeed");

    assert!(response.results);
    mock.assert_async().await;
}

#[tokio::test]
async fn evaluate_preserves_non_success_errors() {
    for status in [401, 403] {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/policy/evaluate")
            .with_status(status)
            .create_async()
            .await;

        let result = client(server.url()).evaluate(&Vec::new()).await;

        assert!(result.is_err(), "status {status} should return an error");
        mock.assert_async().await;
    }
}

fn client(url: String) -> PoaoTilgangClient {
    PoaoTilgangClient::new(
        url,
        "test-scope".to_string(),
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build reqwest client"),
        Arc::new(TokenClientStub::new()),
    )
}

fn match_mock_body(request: &mockito::Request, expected_body: &serde_json::Value) -> bool {
    let mut body: serde_json::Value =
        serde_json::from_slice(request.body().expect("request body should be readable"))
            .expect("request body should be valid JSON");
    let requests = body["requests"]
        .as_array_mut()
        .expect("requests should be an array");
    let mut request_ids = Vec::new();
    for request in requests {
        let request_id = request
            .as_object_mut()
            .expect("each request should be an object")
            .remove("requestId")
            .expect("each request should have requestId");
        let request_id =
            Uuid::parse_str(request_id.as_str().expect("requestId should be a string"))
                .expect("requestId should be a valid UUID");
        assert_eq!(request_id.get_version_num(), 4);
        assert!(
            !request_ids.contains(&request_id),
            "requestIds should be unique"
        );
        request_ids.push(request_id);
    }
    &body == expected_body
}
