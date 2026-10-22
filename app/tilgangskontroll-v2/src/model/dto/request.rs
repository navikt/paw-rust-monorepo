use crate::model::tilgang::TilgangType;
use paw_error_handling::problem_details::ProblemDetails;
use serde::{Deserialize, Serialize};

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattPersonTilgangRequest {
    pub tilgang: TilgangType,
    pub nav_ident: String,
    pub identitetsnummer: String,
}

impl NavAnsattPersonTilgangRequest {
    pub fn validate(&self, path: &str) -> Result<(), ProblemDetails> {
        if self.nav_ident.is_empty() {
            let detail = format!("Felt 'nav_ident' har feil lengde: {}", self.nav_ident.len());
            Err(ProblemDetails::validation_error(path, detail.as_str()))
        } else if self.identitetsnummer.len() != 11 {
            let detail = format!(
                "Felt 'identitetsnummer' har feil lengde: {}",
                self.identitetsnummer.len()
            );
            Err(ProblemDetails::validation_error(path, detail.as_str()))
        } else {
            Ok(())
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavAnsattNavEnhetTilgangRequest {
    pub nav_ident: String,
    pub nav_enhet_id: String,
}

impl NavAnsattNavEnhetTilgangRequest {
    pub fn validate(&self, path: &str) -> Result<(), ProblemDetails> {
        if self.nav_ident.is_empty() {
            let detail = format!("Felt 'nav_ident' har feil lengde: {}", self.nav_ident.len());
            Err(ProblemDetails::validation_error(path, detail.as_str()))
        } else if self.nav_enhet_id.is_empty() {
            let detail = format!(
                "Felt 'nav_enhet_id' har feil lengde: {}",
                self.nav_enhet_id.len()
            );
            Err(ProblemDetails::validation_error(path, detail.as_str()))
        } else {
            Ok(())
        }
    }
}
