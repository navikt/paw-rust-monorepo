use crate::api::query_request::shared_query_request;
use crate::model::dto::request::QueryRequest;
use axum::extract::Request;
use errors::access::AccessError;
use oauth2::principal::Principal;
use paw_error_handling::problem_details::ProblemDetails;
use paw_tilgangskontroll::model::Policy;
use paw_tilgangskontroll_v2_client::client::PawTilgangskontrollV2Client;
use paw_tilgangskontroll_v2_client::model::TilgangType;
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;

pub struct KartleggingPolicy {
    client: Arc<PawTilgangskontrollV2Client>,
}

impl KartleggingPolicy {
    pub fn new(client: Arc<PawTilgangskontrollV2Client>) -> Self {
        Self { client }
    }
}

impl Policy for KartleggingPolicy {
    #[tracing::instrument(skip_all)]
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
                    let nav_ident = nav_ansatt.ident.clone();
                    let (parts, body) = request.into_parts();
                    let query = shared_query_request(&parts)?;

                    match query.as_ref() {
                        QueryRequest::Identitetsnummer(query) => {
                            query.validate(&path)?;
                            let identitetsnummer =
                                Identitetsnummer::new(query.identitetsnummer.clone()).unwrap();
                            let tilgang = self
                                .client
                                .nav_ansatt_tilgang_til_person(
                                    TilgangType::Lese,
                                    nav_ident,
                                    identitetsnummer,
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
                                tracing::info!("Nav-ansatt har tilgang til person (OBO token)");
                                Ok(Request::from_parts(parts, body))
                            } else {
                                tracing::warn!(
                                    "Nav-ansatt har ikke tilgang til person (OBO token)"
                                );
                                let error = format!("Ingen {}-tilgang", TilgangType::Lese.as_ref());
                                Err(ProblemDetails::forbidden(
                                    &path,
                                    AccessError::AccessDenied(error),
                                ))
                            }
                        }
                        QueryRequest::TilknyttetKontor(_) => {
                            tracing::info!("Nav-ansatt har tilgang til kontor (OBO token)");
                            Ok(Request::from_parts(parts, body))
                        }
                    }
                }
                Principal::NavSystem(_nav_system) => {
                    let (parts, body) = request.into_parts();
                    let _query = shared_query_request(&parts)?;
                    tracing::info!("Nav-system har tilgang (M2M token)");
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
                ident: NavIdent::new(NAV_IDENT.to_owned()),
                name: None,
                roles: vec![],
            }));
        request.extensions_mut().insert(Arc::new(query));
        request
    }

    fn client(url: String) -> Arc<PawTilgangskontrollV2Client> {
        Arc::new(PawTilgangskontrollV2Client::new(
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
    async fn tillater_ansatt_lesetilgang_for_person() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
            .match_body(mockito::Matcher::Json(json!({
                "tilgang": TilgangType::Lese,
                "navIdent": NAV_IDENT,
                "identitetsnummer": IDENTITETSNUMMER
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
    async fn avviser_ansatt_lesetilgang_for_person() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
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
    async fn evaluering_feiler_pga_ugyldig_format_for_identitetsnummer() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
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
    async fn evaluering_feiler_pga_500_status_fra_tilgangskontroll() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
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
