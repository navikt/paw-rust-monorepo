use crate::leesah::opplysninger::common::Personnavn;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VergeEllerFullmektig {
    pub navn: Option<Personnavn>,
    pub motparts_personident: Option<String>,
    pub omfang: Option<String>,
    pub omfanget_er_innen_personlig_omraade: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VergemaalEllerFremtidsfullmakt {
    #[serde(rename = "type")]
    pub vergemaal_type: Option<String>,
    pub embete: Option<String>,
    pub verge_eller_fullmektig: VergeEllerFullmektig,
}
