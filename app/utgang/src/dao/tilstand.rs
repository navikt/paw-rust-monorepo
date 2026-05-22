use chrono::{DateTime, Utc};
use interne_hendelser::vo::bruker_type::BrukerType;
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::regler::resultat::{GrunnlagForGodkjenning, Problem};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegelsettVersjon(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stoppet {
    pub tidspunkt: DateTime<Utc>,
    pub utfoert_av: BrukerType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tilstand {
    pub initielle: Vec<Opplysning>,
    pub gjeldende: Option<OpplysningerMedEvaluering>,
    pub forrige: Option<OpplysningerMedEvaluering>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpplysningerMedEvaluering {
    pub opplysninger: Vec<Opplysning>,
    pub tidspunkt: DateTime<Utc>,
    pub evaluering: Option<Evaluering>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evaluering {
    pub regelsett_versjon: RegelsettVersjon,
    pub resultat: EvalueringResultat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvalueringResultat {
    Godkjent {
        grunnlag: Vec<GrunnlagForGodkjenning>,
    },
    Avvist {
        problemer: Vec<Problem>,
    },
}

impl EvalueringResultat {
    pub fn status(&self) -> &'static str {
        match self {
            EvalueringResultat::Godkjent { .. } => "GODKJENT",
            EvalueringResultat::Avvist { .. } => "AVVIST",
        }
    }
}
