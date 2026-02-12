use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::vo::{OpplysningerOmArbeidssoeker, Metadata};

pub const OPPLYSNINGER_OM_ARBEIDSSOEKER_HENDELSE_TYPE: &str = "intern.v1.opplysninger_om_arbeidssoeker";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpplysningerOmArbeidssoekerMottatt {
    pub hendelse_id: Uuid,
    pub id: i64,
    pub identitetsnummer: String,
    pub opplysninger_om_arbeidssoeker: OpplysningerOmArbeidssoeker,
}

impl OpplysningerOmArbeidssoekerMottatt {
    pub fn hendelse_type(&self) -> &'static str {
        OPPLYSNINGER_OM_ARBEIDSSOEKER_HENDELSE_TYPE
    }

    pub fn metadata(&self) -> &Metadata {
        &self.opplysninger_om_arbeidssoeker.metadata
    }
}
