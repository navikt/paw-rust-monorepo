use poao_tilgang_client::client::PoaoTilgangClient;
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;
use types::nav_ident::NavIdent;
use types::nav_kontor_id::NavKontorId;

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
        nav_ident: NavIdent,
        identitetsnummer: Identitetsnummer,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    pub async fn nav_ansatt_tilgang_til_kontor(
        &self,
        nav_ident: NavIdent,
        kontor_id: NavKontorId,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }
}
