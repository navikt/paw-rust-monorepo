use crate::model::dto::request::QueryRequest;
use axum::Json;
use axum::extract::{DefaultBodyLimit, FromRequest, FromRequestParts, Request};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::middleware::{FromFnLayer, Next, from_fn};
use axum::response::Response;
use paw_error_handling::problem_details::ProblemDetails;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tower::Layer;

pub(crate) const QUERY_REQUEST_BODY_LIMIT: usize = 2 * 1024 * 1024;

type BoxedFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type QueryRequestMiddleware = fn(Request, Next) -> BoxedFuture<Result<Response, ProblemDetails>>;
type QueryRequestFromFnLayer = FromFnLayer<QueryRequestMiddleware, (), (Request,)>;

pub(crate) struct SharedQueryRequest(pub Arc<QueryRequest>);

impl<S: Send + Sync> FromRequestParts<S> for SharedQueryRequest {
    type Rejection = ProblemDetails;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        shared_query_request(parts).map(Self)
    }
}

pub(crate) fn shared_query_request(parts: &Parts) -> Result<Arc<QueryRequest>, ProblemDetails> {
    parts
        .extensions
        .get::<Arc<QueryRequest>>()
        .cloned()
        .ok_or_else(|| {
            tracing::error!("Parsed query request is missing");
            ProblemDetails::internal_server_error(
                parts.uri.path(),
                Some("Request-konteksten mangler".to_owned()),
            )
        })
}

pub(crate) fn query_request_layer(limit: usize) -> QueryRequestLayer {
    QueryRequestLayer { limit }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct QueryRequestLayer {
    limit: usize,
}

impl<S> Layer<S> for QueryRequestLayer
where
    S: Clone + Send + Sync + 'static,
{
    type Service =
        <DefaultBodyLimit as Layer<<QueryRequestFromFnLayer as Layer<S>>::Service>>::Service;

    fn layer(&self, inner: S) -> Self::Service {
        DefaultBodyLimit::max(self.limit)
            .layer(from_fn(query_request_middleware_boxed as QueryRequestMiddleware).layer(inner))
    }
}

fn query_request_middleware_boxed(
    request: Request,
    next: Next,
) -> BoxedFuture<Result<Response, ProblemDetails>> {
    Box::pin(parse_query_request(request, next))
}

async fn parse_query_request(request: Request, next: Next) -> Result<Response, ProblemDetails> {
    let (parts, body) = request.into_parts();
    // Json consumes its request; retain the original parts for downstream middleware.
    let extraction_request = Request::from_parts(parts.clone(), body);
    let Json(query) = Json::<QueryRequest>::from_request(extraction_request, &())
        .await
        .map_err(|rejection| {
            if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                tracing::warn!("Query request body exceeds size limit");
                ProblemDetails::payload_too_large(parts.uri.path(), "Request body er for stor")
            } else {
                tracing::warn!("Invalid query request body");
                ProblemDetails::validation_error(parts.uri.path(), "Ugyldig request body")
            }
        })?;
    let mut request = Request::from_parts(parts, axum::body::Body::empty());
    request.extensions_mut().insert(Arc::new(query));
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::{Body, Bytes, to_bytes};
    use axum::http::{HeaderValue, Method, header};
    use axum::routing::post;
    use oauth2::principal::{Anonym, NavSystem, Principal};
    use paw_oauth2_resource_server::middleware::oauth2_server_layer;
    use paw_oauth2_resource_server::state::AuthState;
    use paw_tilgangskontroll::middleware::tilgangskontroll_layer;
    use paw_tilgangskontroll::model::Policy;
    use paw_tilgangskontroll_v2_client::client::PawTilgangskontrollV2Client;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tower::ServiceExt;

    const IDENT: &str = r#"{"type":"IDENTITETSNUMMER","identitetsnummer":"01017012345"}"#;
    const KONTOR: &str = r#"{"type":"TILKNYTTET_KONTOR","kontorId":"1234"}"#;
    const PATH: &str = "/query";

    fn request(body: impl Into<Body>) -> Request {
        Request::builder()
            .method(Method::POST)
            .uri(PATH)
            .header(header::CONTENT_TYPE, "application/json")
            .body(body.into())
            .unwrap()
    }

    fn router(limit: usize) -> Router {
        Router::new()
            .route(
                PATH,
                post(|SharedQueryRequest(_): SharedQueryRequest| async { StatusCode::OK }),
            )
            .route_layer(query_request_layer(limit))
    }

    async fn assert_problem(response: Response, status: StatusCode) {
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/problem+json"
        );
        let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
        let problem: ProblemDetails = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(problem.status, status.as_u16());
        assert_eq!(problem.instance, PATH);
        if status == StatusCode::BAD_REQUEST {
            assert_eq!(problem.detail.as_deref(), Some("Ugyldig request body"));
        }
    }

    struct InspectPolicy(Arc<Mutex<Option<Arc<QueryRequest>>>>);

    impl Policy for InspectPolicy {
        async fn evaluate(&self, request: Request) -> Result<Request, ProblemDetails> {
            assert_eq!(request.method(), Method::POST);
            assert_eq!(request.uri().path(), PATH);
            assert_eq!(request.headers()["x-marker"], "preserved");
            assert!(matches!(
                request.extensions().get::<Principal>(),
                Some(Principal::NavSystem(_))
            ));
            assert_eq!(request.extensions().get::<u64>(), Some(&42));
            let query = request.extensions().get::<Arc<QueryRequest>>().unwrap();
            *self.0.lock().unwrap() = Some(Arc::clone(query));
            Ok(request)
        }
    }

    #[tokio::test]
    async fn shares_same_dto_in_policy_and_repeated_handler_extraction() {
        for body in [IDENT, KONTOR] {
            let observed = Arc::new(Mutex::new(None));
            let handler_observed = Arc::clone(&observed);
            let routes = Router::new()
                .route(
                    PATH,
                    post(move |request: Request| {
                        let observed = Arc::clone(&handler_observed);
                        async move {
                            let (mut parts, body) = request.into_parts();
                            let first = SharedQueryRequest::from_request_parts(&mut parts, &())
                                .await
                                .unwrap();
                            let second = SharedQueryRequest::from_request_parts(&mut parts, &())
                                .await
                                .unwrap();
                            assert!(Arc::ptr_eq(&first.0, &second.0));
                            assert!(Arc::ptr_eq(
                                &first.0,
                                observed.lock().unwrap().as_ref().unwrap()
                            ));
                            assert!(to_bytes(body, 1).await.unwrap().is_empty());
                            Json(first.0.as_ref().clone())
                        }
                    }),
                )
                .route_layer(tilgangskontroll_layer(Arc::new(InspectPolicy(observed))));
            let mut req = request(body);
            req.headers_mut()
                .insert("x-marker", HeaderValue::from_static("preserved"));
            req.extensions_mut().insert(42_u64);
            req.extensions_mut().insert(Principal::NavSystem(NavSystem {
                oid: "test-system".to_owned(),
                roles: vec![],
            }));
            let response = routes
                .route_layer(query_request_layer(QUERY_REQUEST_BODY_LIMIT))
                .oneshot(req)
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
            let actual: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let expected: serde_json::Value = serde_json::from_str(body).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[tokio::test]
    async fn enforces_configured_and_production_limits_at_exact_boundary() {
        for limit in [1024, QUERY_REQUEST_BODY_LIMIT] {
            for (size, status) in [
                (limit, StatusCode::OK),
                (limit + 1, StatusCode::PAYLOAD_TOO_LARGE),
            ] {
                let body = format!("{IDENT}{}", " ".repeat(size - IDENT.len()));
                let response = router(limit).oneshot(request(body)).await.unwrap();
                if status == StatusCode::OK {
                    assert_eq!(response.status(), status);
                } else {
                    assert_problem(response, status).await;
                }
            }
        }
    }

    #[tokio::test]
    async fn rejects_chunked_body_without_content_length() {
        let chunks = futures::stream::iter([
            Ok::<_, std::io::Error>(Bytes::from_static(IDENT.as_bytes())),
            Ok(Bytes::from(vec![b' '; 1024])),
        ]);
        let req = request(Body::from_stream(chunks));
        assert!(!req.headers().contains_key(header::CONTENT_LENGTH));
        assert_problem(
            router(1024).oneshot(req).await.unwrap(),
            StatusCode::PAYLOAD_TOO_LARGE,
        )
        .await;
    }

    #[tokio::test]
    async fn rejects_invalid_json_and_content_type_without_echoing_input() {
        for body in [
            "",
            "{",
            r#"{"type":"UNKNOWN"}"#,
            r#"{"type":"IDENTITETSNUMMER","identitetsnummer":12345678901}"#,
        ] {
            let response = router(1024).oneshot(request(body)).await.unwrap();
            assert_problem(response, StatusCode::BAD_REQUEST).await;
        }
        for content_type in [None, Some("text/plain")] {
            let mut req = request(IDENT);
            req.headers_mut().remove(header::CONTENT_TYPE);
            if let Some(value) = content_type {
                req.headers_mut()
                    .insert(header::CONTENT_TYPE, HeaderValue::from_static(value));
            }
            assert_problem(
                router(1024).oneshot(req).await.unwrap(),
                StatusCode::BAD_REQUEST,
            )
            .await;
        }
        let response = router(1024)
            .oneshot(request(Body::from_stream(futures::stream::iter([Err::<
                Bytes,
                _,
            >(
                std::io::Error::other("sensitive input"),
            )]))))
            .await
            .unwrap();
        assert_problem(response, StatusCode::BAD_REQUEST).await;
    }

    #[tokio::test]
    async fn missing_shared_dto_is_internal_error() {
        let routes = Router::new().route(
            PATH,
            post(|SharedQueryRequest(_): SharedQueryRequest| async { StatusCode::OK }),
        );
        assert_problem(
            routes.oneshot(request(IDENT)).await.unwrap(),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
        .await;
    }

    fn auth_state() -> Arc<AuthState> {
        Arc::new(AuthState {
            azure: None,
            tokenx: None,
            idporten: None,
            maskinporten: None,
        })
    }

    #[tokio::test]
    async fn oauth_rejection_does_not_poll_body() {
        for authorization in [None, Some("Bearer invalid")] {
            let polled = Arc::new(AtomicBool::new(false));
            let observed = Arc::clone(&polled);
            let stream = futures::stream::poll_fn(move |_| {
                observed.store(true, Ordering::SeqCst);
                std::task::Poll::Ready(Some(Ok::<_, std::io::Error>(Bytes::from_static(
                    b"invalid",
                ))))
            });
            let mut req = request(Body::from_stream(stream));
            if let Some(value) = authorization {
                req.headers_mut()
                    .insert(header::AUTHORIZATION, HeaderValue::from_static(value));
            }
            let response = router(1024)
                .route_layer(oauth2_server_layer(auth_state()))
                .oneshot(req)
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
            assert!(!polled.load(Ordering::SeqCst));
        }
    }

    fn kartlegging_policy() -> Arc<crate::logic::security::policy::KartleggingPolicy> {
        Arc::new(crate::logic::security::policy::KartleggingPolicy::new(
            Arc::new(PawTilgangskontrollV2Client::new(
                "http://unused.invalid".to_owned(),
                "unused".to_owned(),
                reqwest::Client::new(),
                Arc::new(token_client_stub::TokenClientStub::new()),
            )),
        ))
    }

    #[tokio::test]
    async fn protected_routes_parse_after_auth_and_before_database_work() {
        use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
        use paw_oauth2_resource_server::oidc::JwksCache;
        use paw_oauth2_resource_server::state::IssuerState;
        use std::collections::HashMap;
        use std::time::{Instant, SystemTime, UNIX_EPOCH};

        let key = uuid::Uuid::new_v4().to_string();
        let header = Header {
            kid: Some("test-key".to_owned()),
            ..Header::default()
        };
        let claims = serde_json::json!({
            "iss": "https://issuer.invalid",
            "aud": "test-client",
            "exp": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() + 3600,
            "oid": "test-system",
            "roles": ["access_as_application"]
        });
        let token = encode(&header, &claims, &EncodingKey::from_secret(key.as_bytes())).unwrap();
        let state = Arc::new(AuthState {
            azure: Some(IssuerState {
                cache: tokio::sync::RwLock::new(JwksCache {
                    keys: HashMap::from([(
                        "test-key".to_owned(),
                        DecodingKey::from_secret(key.as_bytes()),
                    )]),
                    fetched_at: Instant::now(),
                }),
                well_known_url: "http://unused.invalid".to_owned(),
                expected_issuer: "https://issuer.invalid".to_owned(),
                client_id: "test-client".to_owned(),
                http_client: reqwest::Client::new(),
            }),
            tokenx: None,
            idporten: None,
            maskinporten: None,
        });
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@localhost/unused")
            .unwrap();
        let routes = [
            (
                crate::api::v1::kartlegging::API_KARTLEGGING_PATH,
                crate::api::v1::kartlegging::routes(
                    pool.clone(),
                    state.clone(),
                    kartlegging_policy(),
                ),
            ),
            (
                crate::api::v1::arbeidsledighet::API_ARBEIDSLEDIGHET_PATH,
                crate::api::v1::arbeidsledighet::routes(
                    pool.clone(),
                    state.clone(),
                    kartlegging_policy(),
                ),
            ),
        ];
        for (path, router) in routes {
            for (body, status) in [
                ("{".to_owned(), StatusCode::BAD_REQUEST),
                (
                    format!(
                        "{IDENT}{}",
                        " ".repeat(QUERY_REQUEST_BODY_LIMIT + 1 - IDENT.len())
                    ),
                    StatusCode::PAYLOAD_TOO_LARGE,
                ),
            ] {
                let mut req = request(body);
                *req.uri_mut() = path.parse().unwrap();
                req.headers_mut().insert(
                    header::AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
                );
                let response = router.clone().oneshot(req).await.unwrap();
                assert_eq!(response.status(), status);
                let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
                let problem: ProblemDetails = serde_json::from_slice(&bytes).unwrap();
                assert_eq!(problem.instance, path);
                assert_eq!(problem.status, status.as_u16());
            }
        }
    }

    #[tokio::test]
    async fn real_policy_rejects_missing_context_and_never_runs_denied_handler() {
        let policy = kartlegging_policy();
        let mut req = request(IDENT);
        req.extensions_mut().insert(Principal::NavSystem(NavSystem {
            oid: "test".to_owned(),
            roles: vec![],
        }));
        assert_eq!(policy.evaluate(req).await.unwrap_err().status, 500);
        assert_eq!(
            policy.evaluate(request(IDENT)).await.unwrap_err().status,
            403
        );

        for principal in [Principal::NavSystem(NavSystem {
            oid: "test".to_owned(),
            roles: vec![],
        })] {
            let routes = Router::new()
                .route(
                    PATH,
                    post(|SharedQueryRequest(_): SharedQueryRequest| async { StatusCode::OK }),
                )
                .route_layer(tilgangskontroll_layer(Arc::clone(&policy)))
                .route_layer(query_request_layer(1024));
            let mut req = request(IDENT);
            req.extensions_mut().insert(principal);
            assert_eq!(routes.oneshot(req).await.unwrap().status(), StatusCode::OK);
        }

        for (body, status) in [
            (IDENT, StatusCode::FORBIDDEN),
            ("{", StatusCode::BAD_REQUEST),
        ] {
            let called = Arc::new(AtomicBool::new(false));
            let handler_called = Arc::clone(&called);
            let routes = Router::new()
                .route(
                    PATH,
                    post(move || {
                        handler_called.store(true, Ordering::SeqCst);
                        async { StatusCode::OK }
                    }),
                )
                .route_layer(tilgangskontroll_layer(Arc::clone(&policy)));
            let mut req = request(body);
            req.extensions_mut().insert(Principal::Anonym(Anonym));
            assert_problem(
                routes
                    .route_layer(query_request_layer(1024))
                    .oneshot(req)
                    .await
                    .unwrap(),
                status,
            )
            .await;
            assert!(!called.load(Ordering::SeqCst));
        }

        let mut req = request(" ".repeat(1025));
        req.extensions_mut().insert(Principal::Anonym(Anonym));
        let routes = Router::new()
            .route(PATH, post(|| async { StatusCode::OK }))
            .route_layer(tilgangskontroll_layer(policy))
            .route_layer(query_request_layer(1024));
        assert_problem(
            routes.oneshot(req).await.unwrap(),
            StatusCode::PAYLOAD_TOO_LARGE,
        )
        .await;
    }
}
