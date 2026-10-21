use crate::logic::tilgangskontroll::Tilgangskontroll;
use poao_tilgang_client::client::PoaoTilgangClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct RouterState {
    pub tilgangskontroll: Arc<Tilgangskontroll>,
}

impl RouterState {
    pub fn new(poao_tilgang_client: Arc<PoaoTilgangClient>) -> Self {
        Self {
            tilgangskontroll: Arc::new(Tilgangskontroll::new(poao_tilgang_client.clone())),
        }
    }
}
