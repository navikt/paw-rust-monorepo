use crate::fakta::feil::FaktaFeil;
use crate::modell::pdl::{Oppholdstillatelse, Person};
use anyhow::Result;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    BarnFoedtINorgeUtenOppholdstillatelse, HarGyldigOppholdstillatelse,
    IngenInformasjonOmOppholdstillatelse, UkjentStatusForOppholdstillatelse,
};
use regler_core::fakta::UtledeFakta;

#[derive(Debug, Default)]
pub struct UtledeOppholdstillatelseFakta;

impl UtledeFakta<Person, Opplysning> for UtledeOppholdstillatelseFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        if input.opphold.is_empty() {
            Ok(vec![IngenInformasjonOmOppholdstillatelse])
        } else if input.opphold.len() > 1 {
            Err(FaktaFeil::FlereOppholdstillatelser(input.opphold.len()).into())
        } else {
            let opphold = &input.opphold[0];
            let fakta = match opphold.type_ {
                Oppholdstillatelse::Permanent => HarGyldigOppholdstillatelse,
                Oppholdstillatelse::Midlertidig => HarGyldigOppholdstillatelse,
                Oppholdstillatelse::OpplysningMangler => BarnFoedtINorgeUtenOppholdstillatelse,
                Oppholdstillatelse::UkjentVerdi => UkjentStatusForOppholdstillatelse,
            };
            Ok(vec![fakta])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fakta::feil::FaktaFeil;
    use crate::fakta::oppholdstillatelse_fakta::UtledeOppholdstillatelseFakta;
    use crate::modell::pdl::{Opphold, Oppholdstillatelse, Person};
    use interne_hendelser::vo::Opplysning::{
        BarnFoedtINorgeUtenOppholdstillatelse, HarGyldigOppholdstillatelse,
        IngenInformasjonOmOppholdstillatelse, UkjentStatusForOppholdstillatelse,
    };
    use regler_core::fakta::UtledeFakta;

    fn create_person(opphold: Vec<Opphold>) -> Person {
        Person {
            opphold,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_oppholdstillatelse_gir_ingen_info_om_opphold_fakta() {
        let person = create_person(vec![]);
        let result = UtledeOppholdstillatelseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![IngenInformasjonOmOppholdstillatelse]);
    }

    #[test]
    fn mer_enn_en_oppholdstillatelse_gir_flere_opphold_feil() {
        let opphold = vec![Opphold::default(), Opphold::default()];
        let person = create_person(opphold);
        let result = UtledeOppholdstillatelseFakta::default().utlede_fakta(&person);
        match result {
            Ok(fakta) => panic!("Feil resultat: {:?}", fakta),
            Err(err) => assert!(matches!(
                err.downcast_ref::<FaktaFeil>(),
                Some(FaktaFeil::FlereOppholdstillatelser(2))
            )),
        };
    }

    #[test]
    fn en_permanent_oppholdstillatelse_gir_gyldig_opphold_fakta() {
        let opphold = vec![Opphold {
            type_: Oppholdstillatelse::Permanent,
            ..Default::default()
        }];
        let person = create_person(opphold);
        let result = UtledeOppholdstillatelseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![HarGyldigOppholdstillatelse]);
    }

    #[test]
    fn en_midlertidig_oppholdstillatelse_gir_gyldig_opphold_fakta() {
        let opphold = vec![Opphold {
            type_: Oppholdstillatelse::Midlertidig,
            ..Default::default()
        }];
        let person = create_person(opphold);
        let result = UtledeOppholdstillatelseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![HarGyldigOppholdstillatelse]);
    }

    #[test]
    fn opplysninger_mangler_om_oppholdstillatelse_gir_barn_foedt_i_norge_uten_opphold_fakta() {
        let opphold = vec![Opphold {
            type_: Oppholdstillatelse::OpplysningMangler,
            ..Default::default()
        }];
        let person = create_person(opphold);
        let result = UtledeOppholdstillatelseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![BarnFoedtINorgeUtenOppholdstillatelse]);
    }

    #[test]
    fn ukjent_verdi_gir_ukjent_status_for_opphold_fakta() {
        let opphold = vec![Opphold {
            type_: Oppholdstillatelse::UkjentVerdi,
            ..Default::default()
        }];
        let person = create_person(opphold);
        let result = UtledeOppholdstillatelseFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![UkjentStatusForOppholdstillatelse]);
    }
}
