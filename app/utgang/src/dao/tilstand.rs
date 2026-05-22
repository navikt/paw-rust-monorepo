use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tilstand {
    pub initielle: Vec<String>,
    pub gjeldende: Option<OpplysningerMedEvaluering>,
    pub forrige: Option<OpplysningerMedEvaluering>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpplysningerMedEvaluering {
    pub opplysninger: Vec<String>,
    pub tidspunkt: DateTime<Utc>,
    pub evaluering: Option<Evaluering>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evaluering {
    pub regelsett_versjon: String,
    pub resultat: EvalueringResultat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalueringResultat {
    Godkjent {
        grunnlag: Vec<RegelDetalj>,
    },
    Avvist {
        problemer: Vec<ProblemDetalj>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegelDetalj {
    pub regel_id: String,
    pub opplysninger: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProblemDetalj {
    pub regel_id: String,
    pub opplysninger: Vec<String>,
    pub kind: String,
}

impl EvalueringResultat {
    pub fn status(&self) -> &'static str {
        match self {
            EvalueringResultat::Godkjent { .. } => "GODKJENT",
            EvalueringResultat::Avvist { .. } => "AVVIST",
        }
    }
}
