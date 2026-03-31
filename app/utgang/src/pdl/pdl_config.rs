use serde::Deserialize;

pub const BEHANDLINGSNUMMER: &str = "B452";

#[derive(Deserialize, Debug)]
pub struct PDLClientConfig {
    pub target_scope: String,
    pub url: String,
}

impl PDLClientConfig {
    pub fn new(target_scope: String, url: String) -> PDLClientConfig {
        PDLClientConfig {
            target_scope,
            url,
        }
    }
}
