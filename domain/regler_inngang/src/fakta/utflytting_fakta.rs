use crate::modell::pdl::{InnflyttingTilNorge, Person, UtflyttingFraNorge};
use anyhow::Result;
use chrono::NaiveDate;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    IngenFlytteInformasjon, SisteFlyttingVarInnTilNorge, SisteFlyttingVarUtAvNorge,
};
use regler_core::fakta::UtledeFakta;

#[derive(Debug, Default)]
pub struct UtledeUtflyttingFakta;

impl UtledeFakta<Person, Opplysning> for UtledeUtflyttingFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let innflyttinger = &input.innflytting_til_norge;
        let utflyttinger = &input.utflytting_fra_norge;
        if innflyttinger.is_empty() && utflyttinger.is_empty() {
            Ok(vec![IngenFlytteInformasjon])
        } else if !innflyttinger.is_empty() && utflyttinger.is_empty() {
            Ok(vec![SisteFlyttingVarInnTilNorge])
        } else if innflyttinger.is_empty() && !utflyttinger.is_empty() {
            Ok(vec![SisteFlyttingVarUtAvNorge])
        } else {
            let mut flyttinger: Vec<Flytting> = vec![];
            for innflytting in innflyttinger {
                flyttinger.push(Flytting::fra_innflytting(innflytting));
            }
            for utflytting in utflyttinger {
                flyttinger.push(Flytting::fra_utflytting(utflytting));
            }
            Ok(vec![])
        }
    }
}

struct Flytting {
    pub innflytting: bool,
    pub dato: Option<NaiveDate>,
}

impl Flytting {
    fn fra_innflytting(innflytting: &InnflyttingTilNorge) -> Self {
        let tidspunkt = innflytting
            .folkeregistermetadata
            .clone()
            .and_then(|metadata| metadata.ajourholdstidspunkt);
        Self {
            innflytting: true,
            dato: tidspunkt.map(|t| t.naive_local().date()),
        }
    }

    fn fra_utflytting(utflytting: &UtflyttingFraNorge) -> Self {
        Self {
            innflytting: false,
            dato: utflytting.utflyttingsdato,
        }
    }
}

#[cfg(test)]
mod tests {}
