use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TilgangResponse {
    pub har_tilgang: bool,
}

impl TilgangResponse {
    pub fn new(har_tilgang: bool) -> Self {
        Self { har_tilgang }
    }
}
