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
        tilgang_type: TilgangType,
        nav_ident: NavIdent,
        identitetsnummer: Identitetsnummer,
    ) -> anyhow::Result<bool> {
        let nav_ident_str = nav_ident.as_ref();
        let identitetsnummer_str = identitetsnummer.as_ref();

        let policies = match tilgang_type {
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

    pub async fn nav_ansatt_tilgang_til_kontor(
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
