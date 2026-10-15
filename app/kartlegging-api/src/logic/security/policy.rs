use crate::api::query_request::shared_query_request;
use crate::model::dto::request::QueryRequest;
use axum::extract::Request;
use errors::access::AccessError;
use oauth2::principal::Principal;
use paw_error_handling::problem_details::ProblemDetails;
use paw_tilgangskontroll::model::Policy;
use paw_tilgangskontroll_client::client::PawTilgangskontrollClient;
use paw_tilgangskontroll_client::model::TilgangType;
use std::sync::Arc;

pub struct KartleggingPolicy {
    client: Arc<PawTilgangskontrollClient>,
}

impl KartleggingPolicy {
    pub fn new(client: Arc<PawTilgangskontrollClient>) -> Self {
        Self { client }
    }
}

impl Policy for KartleggingPolicy {
    async fn evaluate(&self, request: Request) -> Result<Request, ProblemDetails> {
        let path = request.uri().path().to_owned();
        match request.extensions().get::<Principal>() {
            None => {
                tracing::warn!("No principal found");
                Err(ProblemDetails::forbidden(
                    &path,
                    AccessError::MissingPrincipal,
                ))
            }
            Some(principal) => match principal {
                Principal::NavAnsatt(nav_ansatt) => {
                    tracing::debug!("OBO Access");
                    let nav_ansatt_id = nav_ansatt.ident.as_ref().to_string();
                    let (parts, body) = request.into_parts();
                    let query = shared_query_request(&parts)?;
                    match query.as_ref() {
                        QueryRequest::Identitetsnummer(query) => {
                            query.validate(&path)?;
                            let tilgang = self
                                .client
                                .tilgang(
                                    query.identitetsnummer.clone(),
                                    nav_ansatt_id,
                                    TilgangType::Lese,
                                )
                                .await
                                .map_err(|e| {
                                    tracing::error!("Tilgangskontroll feilet: {}", e);
                                    ProblemDetails::internal_server_error(
                                        &path,
                                        Some("Tilgangskontroll feilet".to_owned()),
                                    )
                                })?;
                            if tilgang.har_tilgang {
                                tracing::debug!("Har tilgang til person");
                                Ok(Request::from_parts(parts, body))
                            } else {
                                tracing::warn!("Har ikke tilgang til person");
                                let error = format!("Ingen {}-tilgang", TilgangType::Lese.as_ref());
                                Err(ProblemDetails::forbidden(
                                    &path,
                                    AccessError::AccessDenied(error),
                                ))
                            }
                        }
                        QueryRequest::TilknyttetKontor(_) => {
                            tracing::debug!("Har tilgang til kontor");
                            Ok(Request::from_parts(parts, body))
                        }
                    }
                }
                Principal::NavSystem(_nav_system) => {
                    let (parts, body) = request.into_parts();
                    let _query = shared_query_request(&parts)?;
                    tracing::debug!("M2M Access");
                    Ok(Request::from_parts(parts, body))
                }
                _ => {
                    tracing::warn!("Incorrect principal");
                    Err(ProblemDetails::forbidden(
                        &path,
                        AccessError::IncorrectPrincipal,
                    ))
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use oauth2::principal::NavAnsatt;
    use paw_tilgangskontroll_client::model::TilgangType;
    use serde_json::json;
    use types::nav_ident::NavIdent;

    const PATH: &str = "/api/v1/kartlegging";
    const IDENTITETSNUMMER: &str = "01017012345";
    const NAV_IDENT: &str = "Z123456";

    fn request(query: QueryRequest) -> Request {
        let mut request = Request::new(Body::empty());
        *request.uri_mut() = PATH.parse().unwrap();
        request
            .extensions_mut()
            .insert(Principal::NavAnsatt(NavAnsatt {
                oid: "employee-oid".to_owned(),
                ident: NavIdent::new(NAV_IDENT.to_owned()).unwrap(),
                name: None,
                roles: vec![],
            }));
        request.extensions_mut().insert(Arc::new(query));
        request
    }

    fn client(url: String) -> Arc<PawTilgangskontrollClient> {
        Arc::new(PawTilgangskontrollClient::new(
            url,
            "unused".to_owned(),
            reqwest::Client::new(),
            Arc::new(token_client_stub::TokenClientStub::new()),
        ))
    }

    fn identitetsnummer_query(identitetsnummer: &str) -> QueryRequest {
        serde_json::from_value(json!({
            "type": "IDENTITETSNUMMER",
            "identitetsnummer": identitetsnummer
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn allows_employee_when_access_check_grants_read_access() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang")
            .match_body(mockito::Matcher::Json(json!({
                "identitetsnummer": IDENTITETSNUMMER,
                "navAnsattId": NAV_IDENT,
                "tilgang": TilgangType::Lese
            })))
            .with_status(200)
            .with_body(r#"{"harTilgang":true}"#)
            .expect(1)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        assert!(
            policy
                .evaluate(request(identitetsnummer_query(IDENTITETSNUMMER)))
                .await
                .is_ok()
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn rejects_employee_when_access_check_denies_read_access() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang")
            .with_status(200)
            .with_body(r#"{"harTilgang":false}"#)
            .expect(1)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(identitetsnummer_query(IDENTITETSNUMMER)))
            .await
            .unwrap_err();

        assert_eq!(error.status, 403);
        assert_eq!(error.instance, PATH);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn rejects_invalid_identity_without_calling_access_service() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang")
            .expect(0)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(identitetsnummer_query("invalid")))
            .await
            .unwrap_err();

        assert_eq!(error.status, 400);
        assert_eq!(error.instance, PATH);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn reports_access_service_failure_with_request_path() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang")
            .with_status(503)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(identitetsnummer_query(IDENTITETSNUMMER)))
            .await
            .unwrap_err();

        assert_eq!(error.status, 500);
        assert_eq!(error.instance, PATH);
        assert_eq!(error.detail.as_deref(), Some("Tilgangskontroll feilet"));
        mock.assert_async().await;
    }
}
