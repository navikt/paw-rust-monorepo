use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtenlandskAdresse {
    pub adressenavn_nummer: Option<String>,
    pub bygning_etasje_leilighet: Option<String>,
    pub postboks_nummer_navn: Option<String>,
    pub postkode: Option<String>,
    pub by_sted: Option<String>,
    pub region_distrikt_omraade: Option<String>,
    pub landkode: String,
}
