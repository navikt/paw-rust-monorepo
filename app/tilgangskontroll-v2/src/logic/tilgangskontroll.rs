use crate::model::tilgang::TilgangType;
use poao_tilgang_client::client::PoaoTilgangClient;
use poao_tilgang_client::model::{EvaluatePoliciesRequest, Policy, PolicyRequest};
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;
use types::nav_enhet_id::NavEnhetId;
use types::nav_ident::NavIdent;

pub struct Tilgangskontroll {
    poao_tilgang_client: Arc<PoaoTilgangClient>,
}

impl Tilgangskontroll {
    pub fn new(poao_tilgang_client: Arc<PoaoTilgangClient>) -> Self {
        Self {
            poao_tilgang_client,
        }
    }

    pub async fn nav_ansatt_tilgang_til_person(
        &self,
        tilgang: TilgangType,
        nav_ident: NavIdent,
        identitetsnummer: Identitetsnummer,
    ) -> anyhow::Result<bool> {
        let nav_ident_str = nav_ident.as_ref();
        let identitetsnummer_str = identitetsnummer.as_ref();

        let policies = match tilgang {
            TilgangType::Lese => {
                vec![Policy::nav_ansatt_lesetilgang_til_ekstern_bruker_v1(
                    nav_ident_str.to_string(),
                    identitetsnummer_str.to_string(),
                )]
            }
            TilgangType::Skrive => {
                vec![Policy::nav_ansatt_skrivetilgang_til_ekstern_bruker_v1(
                    nav_ident_str.to_string(),
                    identitetsnummer_str.to_string(),
                )]
            }
            TilgangType::LeseSkrive => {
                vec![
                    Policy::nav_ansatt_lesetilgang_til_ekstern_bruker_v1(
                        nav_ident_str.to_string(),
                        identitetsnummer_str.to_string(),
                    ),
                    Policy::nav_ansatt_skrivetilgang_til_ekstern_bruker_v1(
                        nav_ident_str.to_string(),
                        identitetsnummer_str.to_string(),
                    ),
                ]
            }
        };
        self.tilgang(policies).await
    }

    pub async fn nav_ansatt_tilgang_til_enhet(
        &self,
        nav_ident: NavIdent,
        nav_enhet_id: NavEnhetId,
    ) -> anyhow::Result<bool> {
        let nav_ident_str = nav_ident.as_ref();
        let nav_enhet_id_str = nav_enhet_id.as_ref();

        let policies = vec![Policy::nav_ansatt_tilgang_til_nav_enhet_v1(
            nav_ident_str.to_string(),
            nav_enhet_id_str.to_string(),
        )];
        self.tilgang(policies).await
    }

    async fn tilgang(&self, policies: Vec<Policy>) -> anyhow::Result<bool> {
        let policy_requests = policies
            .iter()
            .map(|policy| PolicyRequest::new(policy))
            .collect();
        let request = EvaluatePoliciesRequest::new(policy_requests);
        let response = self.poao_tilgang_client.evaluate(request).await?;
        Ok(response.is_permitted())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use poao_tilgang_client::error::PoaoTilgangClientError;
    use serde_json::{Value, json};
    use token_client_stub::TokenClientStub;
    use uuid::Uuid;

    const LESE_POLICY: &str = "NAV_ANSATT_NAV_IDENT_LESETILGANG_TIL_EKSTERN_BRUKER_V1";
    const SKRIVE_POLICY: &str = "NAV_ANSATT_NAV_IDENT_SKRIVETILGANG_TIL_EKSTERN_BRUKER_V1";

    #[tokio::test]
    async fn skal_sjekke_lesetilgang_til_person() {
        sjekk_person_tilgang(TilgangType::Lese, &[LESE_POLICY]).await;
    }

    #[tokio::test]
    async fn skal_sjekke_skrivetilgang_til_person() {
        sjekk_person_tilgang(TilgangType::Skrive, &[SKRIVE_POLICY]).await;
    }

    #[tokio::test]
    async fn skal_kreve_baade_lese_og_skrivetilgang_til_person() {
        sjekk_person_tilgang(TilgangType::LeseSkrive, &[LESE_POLICY, SKRIVE_POLICY]).await;
    }

    async fn sjekk_person_tilgang(tilgang_type: TilgangType, policy_ids: &[&str]) {
        let decisions = if policy_ids.len() == 2 {
            vec![
                vec!["PERMIT", "PERMIT"],
                vec!["PERMIT", "DENY"],
                vec!["DENY", "PERMIT"],
                vec!["DENY", "DENY"],
            ]
        } else {
            vec![vec!["PERMIT"], vec!["DENY"]]
        };
        for decisions in decisions {
            let mut server = mockito::Server::new_async().await;
            let requests: Vec<_> = policy_ids
                .iter()
                .map(|policy_id| {
                    json!({
                        "policyId": policy_id,
                        "policyInput": {
                            "navIdent": "Z123456",
                            "norskIdent": "01017012345"
                        }
                    })
                })
                .collect();
            let mock =
                mock_evaluering(&mut server, json!({ "requests": requests }), &decisions).await;

            let tilgang = tilgangskontroll(server.url())
                .nav_ansatt_tilgang_til_person(
                    tilgang_type.clone(),
                    NavIdent::new("Z123456".to_string()),
                    Identitetsnummer::new("01017012345".to_string()).unwrap(),
                )
                .await;

            mock.assert_async().await;
            assert_eq!(
                tilgang.expect("tilgangssjekk skal lykkes"),
                decisions.iter().all(|decision| *decision == "PERMIT"),
                "tilgangstype {tilgang_type:?}, beslutninger {decisions:?}"
            );
        }
    }

    #[tokio::test]
    async fn skal_sjekke_tilgang_til_nav_enhet() {
        for (decision, forventet_tilgang) in [("PERMIT", true), ("DENY", false)] {
            let mut server = mockito::Server::new_async().await;
            let mock = mock_evaluering(
                &mut server,
                json!({
                    "requests": [{
                        "policyId": "NAV_ANSATT_NAV_IDENT_TILGANG_TIL_NAV_ENHET_V1",
                        "policyInput": {
                            "navIdent": "Z123456",
                            "navEnhetId": "0301"
                        }
                    }]
                }),
                &[decision],
            )
            .await;

            let tilgang = tilgangskontroll(server.url())
                .nav_ansatt_tilgang_til_enhet(
                    NavIdent::new("Z123456".to_string()),
                    NavEnhetId::new("0301".to_string()),
                )
                .await
                .expect("tilgangssjekk skal lykkes");

            assert_eq!(tilgang, forventet_tilgang, "beslutning {decision}");
            mock.assert_async().await;
        }
    }

    #[tokio::test]
    async fn skal_propagere_http_feil() {
        for status in [401, 403, 500] {
            for person_tilgang in [true, false] {
                let mut server = mockito::Server::new_async().await;
                let mock = server
                    .mock("POST", "/api/v1/policy/evaluate")
                    .with_status(status)
                    .create_async()
                    .await;

                let error = sjekk_tilgang(&tilgangskontroll(server.url()), person_tilgang)
                    .await
                    .expect_err("HTTP-feil skal ikke bli en tilgangsbeslutning");

                let error = error.downcast_ref::<PoaoTilgangClientError>().unwrap();
                match status {
                    401 => assert!(matches!(error, PoaoTilgangClientError::NotAuthorized)),
                    403 => assert!(matches!(
                        error,
                        PoaoTilgangClientError::AuthenticationFailed
                    )),
                    500 => assert!(matches!(error, PoaoTilgangClientError::UnknownError(_))),
                    _ => unreachable!(),
                }
                mock.assert_async().await;
            }
        }
    }

    #[tokio::test]
    async fn skal_propagere_ugyldig_respons() {
        for person_tilgang in [true, false] {
            let mut server = mockito::Server::new_async().await;
            let mock = server
                .mock("POST", "/api/v1/policy/evaluate")
                .with_status(200)
                .with_body("not json")
                .create_async()
                .await;

            let error = sjekk_tilgang(&tilgangskontroll(server.url()), person_tilgang)
                .await
                .expect_err("ugyldig respons skal ikke bli en tilgangsbeslutning");

            assert!(error.downcast_ref::<reqwest::Error>().unwrap().is_decode());
            mock.assert_async().await;
        }
    }

    async fn sjekk_tilgang(
        tilgangskontroll: &Tilgangskontroll,
        person_tilgang: bool,
    ) -> anyhow::Result<bool> {
        let nav_ident = NavIdent::new("Z123456".to_string());
        if person_tilgang {
            tilgangskontroll
                .nav_ansatt_tilgang_til_person(
                    TilgangType::Skrive,
                    nav_ident,
                    Identitetsnummer::new("01017012345".to_string()).unwrap(),
                )
                .await
        } else {
            tilgangskontroll
                .nav_ansatt_tilgang_til_enhet(nav_ident, NavEnhetId::new("0301".to_string()))
                .await
        }
    }

    async fn mock_evaluering(
        server: &mut mockito::Server,
        expected_body: Value,
        decisions: &[&str],
    ) -> mockito::Mock {
        let results: Vec<_> = decisions
            .iter()
            .map(|decision| {
                json!({
                    "requestId": Uuid::new_v4(),
                    "decision": { "type": decision }
                })
            })
            .collect();
        server
            .mock("POST", "/api/v1/policy/evaluate")
            .match_request(move |request| {
                let mut body: Value = serde_json::from_slice(request.body().unwrap()).unwrap();
                for request in body["requests"].as_array_mut().unwrap() {
                    request.as_object_mut().unwrap().remove("requestId");
                }
                body == expected_body
            })
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({ "results": results }).to_string())
            .create_async()
            .await
    }

    fn tilgangskontroll(url: String) -> Tilgangskontroll {
        Tilgangskontroll::new(Arc::new(PoaoTilgangClient::new(
            url,
            "test-scope".to_string(),
            reqwest::Client::builder().no_proxy().build().unwrap(),
            Arc::new(TokenClientStub::new()),
        )))
    }
}
