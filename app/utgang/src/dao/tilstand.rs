use chrono::{DateTime, Utc};
use interne_hendelser::vo::Opplysning;
use regler_arbeidssoeker::{regelmotor::Evaluering, regler::regelsett::Evalueringsresultat};
use serde::{Deserialize, Serialize};

use crate::{
    dao::regel_evaluering::{RegelEvaluering, Status},
    kafka::periode_deserializer::BrukerType,
};

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

impl Tilstand {
    pub fn registrer_evaluering(
        self,
        tidspunkt: chrono::DateTime<Utc>,
        evaluering: Evaluering,
    ) -> Self {
        let (status, regel_ideer) = match evaluering.resultat {
            Evalueringsresultat::Godkjent { regel_ider } => (Status::Godkjent, regel_ider),
            Evalueringsresultat::Avvist { regel_ider } => (Status::Avvist, regel_ider),
            Evalueringsresultat::KreverManuellVurdering { regel_ider } => {
                (Status::KreverManuellVurdering, regel_ider)
            }
        };
        let regel_evaluering = RegelEvaluering {
            tidspunkt,
            regelsett_versjon: evaluering.regelsett_versjon.into_string(),
            status,
            regel_ider: regel_ideer.into_iter().map(|id| id.into_string()).collect(),
        };
        let opplysning_med_evaluering = OpplysningerMedEvaluering {
            opplysninger: evaluering.fakta,
            oppdatert: tidspunkt,
            evaluering: Some(regel_evaluering),
        };
        Tilstand {
            initielle: self.initielle,
            gjeldende: Some(opplysning_med_evaluering),
            forrige: self.gjeldende,
        }
    }

    pub fn endringer(&self) -> Vec<Endring> {
        let mut endringer = Vec::new();

        let forrige = self.forrige.as_ref();
        let gjeldende = self.gjeldende.as_ref();
        let forrige_eval = forrige.and_then(|o| o.evaluering.as_ref());
        let gjeldende_eval = gjeldende.and_then(|o| o.evaluering.as_ref());
        let forrige_oppl = forrige.map(|o| &o.opplysninger);
        let gjeldende_oppl = gjeldende.map(|o| &o.opplysninger);

        match (forrige_eval, gjeldende_eval, forrige_oppl, gjeldende_oppl) {
            (
                Some(forige_eval),
                Some(gjeldene_eval),
                Some(forrige_opplysninger),
                Some(gjeldene_opplysninger),
            ) => {
                if forige_eval.regelsett_versjon != gjeldene_eval.regelsett_versjon {
                    endringer.push(Endring::RegelsettEndret {
                        forrige: forige_eval.regelsett_versjon.clone(),
                        gjeldende: gjeldene_eval.regelsett_versjon.clone(),
                    });
                }
                if !eq_unordered(&forige_eval.regel_ider, &gjeldene_eval.regel_ider) {
                    endringer.push(Endring::RegelIderEndret {
                        forrige: forige_eval.regel_ider.clone(),
                        gjeldende: gjeldene_eval.regel_ider.clone(),
                    });
                }
                if forige_eval.status != gjeldene_eval.status {
                    endringer.push(Endring::StatusEndret {
                        forrige: forige_eval.status.clone(),
                        gjeldende: gjeldene_eval.status.clone(),
                    });
                }
                if forrige_opplysninger != gjeldene_opplysninger {
                    endringer.push(Endring::OpplysningerEndret {
                        forrige: forrige_opplysninger.clone(),
                        gjeldende: gjeldene_opplysninger.clone(),
                    });
                }
            }
            _ => {}
        }

        endringer
    }
}

fn eq_unordered<T: PartialEq>(a: &[T], b: &[T]) -> bool {
    a.len() == b.len() && a.iter().all(|item| b.contains(item))
}

pub enum Endring {
    StatusEndret { forrige: Status, gjeldende: Status },
    RegelIderEndret { forrige: Vec<String>, gjeldende: Vec<String> },
    RegelsettEndret { forrige: String, gjeldende: String },
    OpplysningerEndret { forrige: Vec<Opplysning>, gjeldende: Vec<Opplysning> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpplysningerMedEvaluering {
    pub opplysninger: Vec<Opplysning>,
    pub oppdatert: DateTime<Utc>,
    pub evaluering: Option<RegelEvaluering>,
}
