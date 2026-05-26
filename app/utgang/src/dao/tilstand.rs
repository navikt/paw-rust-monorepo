use chrono::{DateTime, Utc};
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::regelmotor::Evaluering;
use serde::{Deserialize, Serialize};

use crate::kafka::periode_deserializer::BrukerType;

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
