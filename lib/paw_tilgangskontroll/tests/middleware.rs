use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::StatusCode;
use axum::routing::post;
use errors::access::AccessError;
use oauth2::principal::{NavSystem, Principal};
use paw_error_handling::problem_details::ProblemDetails;
use paw_tilgangskontroll::{Policy, tilgangskontroll};
use tower::ServiceExt;

struct PrincipalCheckPolicy;

impl Policy for PrincipalCheckPolicy {
    async fn evaluate(&self, request: Request) -> Result<Request, ProblemDetails> {
        match request.extensions().get::<Principal>() {
            None => Err(ProblemDetails::forbidden(
                "/test",
                AccessError::MissingPrincipal,
            )),
            Some(principal) => match principal {
                Principal::NavAnsatt(nav_ansatt) => {
                    if nav_ansatt.roles.contains(&"yolo".to_string()) {
                        Ok(request)
                    } else {
                        Err(ProblemDetails::forbidden(
                            "/test",
                            AccessError::InsufficientRoles("yolo".to_string()),
                        ))
                    }
                }
                _ => Err(ProblemDetails::forbidden(
                    "/test",
                    AccessError::IncorrectPrincipal,
                )),
            },
        }
    }
}

struct ExternalCallPolicy;

impl Policy for ExternalCallPolicy {
    async fn evaluate(&self, _request: Request) -> Result<Request, ProblemDetails> {
        todo!()
    }
}

fn test_routes<P: Policy>(policy: P) -> Router {
    Router::new()
        .route(
            "/test",
            post(move |request: Request| async move {
                to_bytes(request.into_body(), 256).await.unwrap()
            }),
        )
        .route_layer(tilgangskontroll(policy))
}

fn test_request(principal: Option<Principal>, body: &str) -> Request {
    let mut request = Request::builder()
        .method("POST")
        .uri("/test")
        .body(Body::from(body.to_string()))
        .unwrap();
    request.extensions_mut().insert(42usize);
    if let Some(principal) = principal {
        request.extensions_mut().insert(principal);
    }
    request
}

const BODY: &str = r#"{"resource":"test"}"#;

#[tokio::test]
async fn missing_principal_returns_403_error() {
    let request = test_request(None, BODY);
    let response = test_routes(PrincipalCheckPolicy)
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body();
    let bytes = to_bytes(body, 1024).await.unwrap();
    let error: ProblemDetails = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(error.status, StatusCode::FORBIDDEN.as_u16());
    assert_eq!(error.problem_type, "urn:paw:http:forbidden".to_string());
    assert_eq!(
        error.detail,
        Some(AccessError::MissingPrincipal.to_string())
    );
}

#[tokio::test]
async fn incorrect_principal_returns_403_error() {
    let request = test_request(
        Some(Principal::NavSystem(NavSystem {
            oid: "oid".to_string(),
            roles: vec![],
        })),
        BODY,
    );
    let response = test_routes(PrincipalCheckPolicy)
        .oneshot(request)
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let body = response.into_body();
    let bytes = to_bytes(body, 1024).await.unwrap();
    let error: ProblemDetails = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(error.status, StatusCode::FORBIDDEN.as_u16());
    assert_eq!(error.problem_type, "urn:paw:http:forbidden".to_string());
    assert_eq!(
        error.detail,
        Some(AccessError::IncorrectPrincipal.to_string())
    );
}
