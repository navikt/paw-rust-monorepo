use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{Metadata, Utdanning, Helse, Jobbsituasjon, Annet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpplysningerOmArbeidssoeker {
    pub id: Uuid,
    pub metadata: Metadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utdanning: Option<Utdanning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub helse: Option<Helse>,
    pub jobbsituasjon: Jobbsituasjon,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annet: Option<Annet>,
}
