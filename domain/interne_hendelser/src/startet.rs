use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashSet;

use crate::vo::{Metadata, Opplysning};

pub const STARTET_HENDELSE_TYPE: &str = "intern.v1.startet";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Startet {
    pub hendelse_id: Uuid,
    pub id: i64,
    pub identitetsnummer: String,
    pub metadata: Metadata,
    #[serde(default)]
    pub opplysninger: HashSet<Opplysning>,
}

impl Startet {
    pub fn hendelse_type(&self) -> &'static str {
        STARTET_HENDELSE_TYPE
    }
}

impl crate::Hendelse for Startet {
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
