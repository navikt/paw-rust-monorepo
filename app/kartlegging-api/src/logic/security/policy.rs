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
use types::nav_enhet_id::NavEnhetId;

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
        match request.extensions().get::<Principal>() {
            None => {
                tracing::warn!("No principal found");
                let path = request.uri().path().to_owned();
                Err(ProblemDetails::forbidden(
                    &path,
                    AccessError::MissingPrincipal,
                ))
            }
            Some(principal) => match principal {
                Principal::NavAnsatt(nav_ansatt) => {
                    let path = request.uri().path().to_owned();
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
                                    tracing::error!("Tilgangskontroll feilet: {}", e.to_string());
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
                                    "Nav-ansatt har ikke {}-tilgang til person (OBO token)",
                                    TilgangType::Lese.as_ref()
                                );
                                let error = format!(
                                    "Nav-ansatt har ikke {}-tilgang til person",
                                    TilgangType::Lese.as_ref()
                                );
                                Err(ProblemDetails::forbidden(
                                    &path,
                                    AccessError::AccessDenied(error),
                                ))
                            }
                        }
                        QueryRequest::TilknyttetKontor(query) => {
                            query.validate(&path)?;
                            let nav_enhet_id = NavEnhetId::new(query.kontor_id.clone());
                            let tilgang = self
                                .client
                                .nav_ansatt_tilgang_til_enhet(nav_ident, nav_enhet_id)
                                .await
                                .map_err(|e| {
                                    tracing::error!("Tilgangskontroll feilet: {}", e.to_string());
                                    ProblemDetails::internal_server_error(
                                        &path,
                                        Some("Tilgangskontroll feilet".to_owned()),
                                    )
                                })?;
                            if tilgang.har_tilgang {
                                tracing::info!("Nav-ansatt har tilgang til enhet (OBO token)");
                                Ok(Request::from_parts(parts, body))
                            } else {
                                tracing::warn!("Nav-ansatt har ikke tilgang til enhet (OBO token)");
                                Err(ProblemDetails::forbidden(
                                    &path,
                                    AccessError::AccessDenied(
                                        "Nav-ansatt har ikke tilgang til enhet".to_string(),
                                    ),
                                ))
                            }
                        }
                    }
                }
                Principal::NavSystem(_nav_system) => {
                    let path = request.uri().path().to_owned();
                    let (parts, body) = request.into_parts();
                    let query = shared_query_request(&parts)?;
                    match query.as_ref() {
                        QueryRequest::Identitetsnummer(query) => {
                            query.validate(&path)?;
                            tracing::info!("Nav-system har tilgang til person (M2M token)");
                        }
                        QueryRequest::TilknyttetKontor(query) => {
                            tracing::info!("Nav-system har tilgang til enhet (M2M token)");
                            query.validate(&path)?;
                        }
                    }
                    Ok(Request::from_parts(parts, body))
                }
                _ => {
                    tracing::warn!("Incorrect principal");
                    let path = request.uri().path().to_owned();
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
    use crate::model::dto::request::QueryRequest::{Identitetsnummer, TilknyttetKontor};
    use crate::model::dto::request::{IdentitetsnummerQueryRequest, TilknyttetKontorQueryRequest};
    use axum::body::Body;
    use oauth2::principal::NavAnsatt;
    use paw_tilgangskontroll_v2_client::model::{
        NavAnsattNavEnhetTilgangRequest, NavAnsattPersonTilgangRequest, TilgangResponse,
    };
    use types::nav_ident::NavIdent;

    const PATH: &str = "/api/v1/kartlegging";
    const IDENTITETSNUMMER: &str = "01017012345";
    const NAV_IDENT: &str = "Z123456";
    const NAV_ENHET_ID: &str = "1234";

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

    #[tokio::test]
    async fn ansatt_lesetilgang_for_person_tillates() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
            .match_body(mockito::Matcher::Json(
                serde_json::to_value(NavAnsattPersonTilgangRequest::new(
                    TilgangType::Lese,
                    NAV_IDENT.to_string(),
                    IDENTITETSNUMMER.to_string(),
                ))
                .unwrap(),
            ))
            .with_status(200)
            .with_body(serde_json::to_string(&TilgangResponse::new(true)).unwrap())
            .expect(1)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        assert!(
            policy
                .evaluate(request(Identitetsnummer(IdentitetsnummerQueryRequest {
                    identitetsnummer: IDENTITETSNUMMER.to_string(),
                    paging: None,
                })))
                .await
                .is_ok()
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_lesetilgang_for_person_avvises() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
            .with_status(200)
            .with_body(serde_json::to_string(&TilgangResponse::new(false)).unwrap())
            .expect(1)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(Identitetsnummer(IdentitetsnummerQueryRequest {
                identitetsnummer: IDENTITETSNUMMER.to_string(),
                paging: None,
            })))
            .await
            .unwrap_err();

        assert_eq!(error.status, 403);
        assert_eq!(error.instance, PATH);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_lesetilgang_for_person_feiler_pga_ugyldig_format_for_identitetsnummer() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
            .expect(0)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(Identitetsnummer(IdentitetsnummerQueryRequest {
                identitetsnummer: "invalid".to_string(),
                paging: None,
            })))
            .await
            .unwrap_err();

        assert_eq!(error.status, 400);
        assert_eq!(error.instance, PATH);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_lesetilgang_for_person_feiler_pga_503_status_fra_tilgangskontroll() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/person")
            .with_status(503)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(Identitetsnummer(IdentitetsnummerQueryRequest {
                identitetsnummer: IDENTITETSNUMMER.to_string(),
                paging: None,
            })))
            .await
            .unwrap_err();

        assert_eq!(error.status, 500);
        assert_eq!(error.instance, PATH);
        assert_eq!(error.detail.as_deref(), Some("Tilgangskontroll feilet"));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_tilgang_for_enhet_tillates() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/enhet")
            .match_body(mockito::Matcher::Json(
                serde_json::to_value(NavAnsattNavEnhetTilgangRequest::new(
                    NAV_IDENT.to_string(),
                    NAV_ENHET_ID.to_string(),
                ))
                .unwrap(),
            ))
            .with_status(200)
            .with_body(serde_json::to_string(&TilgangResponse::new(true)).unwrap())
            .expect(1)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        assert!(
            policy
                .evaluate(request(TilknyttetKontor(TilknyttetKontorQueryRequest {
                    kontor_id: NAV_ENHET_ID.to_string(),
                    kontor_type: None,
                    ledig_siden: None,
                    paging: None,
                })))
                .await
                .is_ok()
        );
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_tilgang_for_enhet_avvises() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/enhet")
            .with_status(200)
            .with_body(serde_json::to_string(&TilgangResponse::new(false)).unwrap())
            .expect(1)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(TilknyttetKontor(TilknyttetKontorQueryRequest {
                kontor_id: NAV_ENHET_ID.to_string(),
                kontor_type: None,
                ledig_siden: None,
                paging: None,
            })))
            .await
            .unwrap_err();

        assert_eq!(error.status, 403);
        assert_eq!(error.instance, PATH);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_tilgang_for_enhet_feiler_pga_ugyldig_format_for_identitetsnummer() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/enhet")
            .expect(0)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(TilknyttetKontor(TilknyttetKontorQueryRequest {
                kontor_id: "".to_string(),
                kontor_type: None,
                ledig_siden: None,
                paging: None,
            })))
            .await
            .unwrap_err();

        assert_eq!(error.status, 400);
        assert_eq!(error.instance, PATH);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn ansatt_tilgang_for_enhet_feiler_pga_412_status_fra_tilgangskontroll() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/api/v1/tilgang/enhet")
            .with_status(412)
            .create_async()
            .await;
        let policy = KartleggingPolicy::new(client(server.url()));

        let error = policy
            .evaluate(request(TilknyttetKontor(TilknyttetKontorQueryRequest {
                kontor_id: NAV_ENHET_ID.to_string(),
                kontor_type: None,
                ledig_siden: None,
                paging: None,
            })))
            .await
            .unwrap_err();

        assert_eq!(error.status, 500);
        assert_eq!(error.instance, PATH);
        assert_eq!(error.detail.as_deref(), Some("Tilgangskontroll feilet"));
        mock.assert_async().await;
    }
}
