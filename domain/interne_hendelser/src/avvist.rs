use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;

use crate::vo::{Metadata, Opplysning};

pub const AVVIST_HENDELSE_TYPE: &str = "intern.v1.avvist";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avvist {
    pub hendelse_id: Uuid,
    pub id: i64,
    pub identitetsnummer: String,
    pub metadata: Metadata,
    #[serde(default)]
    pub opplysninger: HashSet<Opplysning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handling: Option<String>,
}

impl Avvist {
    pub fn hendelse_type(&self) -> &'static str {
        AVVIST_HENDELSE_TYPE
    }
}

impl crate::Hendelse for Avvist {
    fn hendelse_id(&self) -> Uuid {
        self.hendelse_id
    }
    fn id(&self) -> i64 {
        self.id
    }
    fn identitetsnummer(&self) -> &str {
        &self.identitetsnummer
    }
    fn metadata(&self) -> &Metadata {
        &self.metadata
    }
    fn opplysninger(&self) -> &HashSet<Opplysning> {
        &self.opplysninger
    }
}
