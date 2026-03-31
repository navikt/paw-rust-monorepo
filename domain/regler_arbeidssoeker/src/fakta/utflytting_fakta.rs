use crate::fakta::UtledeFakta;
use anyhow::Result;
use chrono::NaiveDate;
use interne_hendelser::vo::Opplysning;
use interne_hendelser::vo::Opplysning::{
    IkkeMuligAaIdentifisereSisteFlytting, IngenFlytteInformasjon, SisteFlyttingVarInnTilNorge,
    SisteFlyttingVarUtAvNorge,
};
use pdl_graphql::pdl::{InnflyttingTilNorge, Person, UtflyttingFraNorge};

#[derive(Debug, Default)]
pub struct UtledeUtflyttingFakta;

impl UtledeFakta<Person, Opplysning> for UtledeUtflyttingFakta {
    fn utlede_fakta(&self, input: &Person) -> Result<Vec<Opplysning>> {
        let innflyttinger = input
            .innflytting_til_norge
            .iter()
            .map(|i| Flytting::fra_innflytting(i.clone()))
            .collect::<Vec<Flytting>>();
        let utflyttinger = input
            .utflytting_fra_norge
            .iter()
            .map(|u| Flytting::fra_utflytting(u.clone()))
            .collect::<Vec<Flytting>>();
        if innflyttinger.is_empty() && utflyttinger.is_empty() {
            Ok(vec![IngenFlytteInformasjon])
        } else if !innflyttinger.is_empty() && utflyttinger.is_empty() {
            Ok(vec![SisteFlyttingVarInnTilNorge])
        } else if innflyttinger.is_empty() && !utflyttinger.is_empty() {
            Ok(vec![SisteFlyttingVarUtAvNorge])
        } else {
            let flyttinger = vec![innflyttinger, utflyttinger].concat();
            let mut unike_datoer = flyttinger
                .iter()
                .filter_map(|f| f.dato)
                .collect::<Vec<NaiveDate>>();
            unike_datoer.dedup(); // Fjern duplikate datoer
            if unike_datoer.len() < 2 {
                Ok(vec![IkkeMuligAaIdentifisereSisteFlytting])
            } else if flyttinger.iter().any(|f| f.dato.is_none()) {
                Ok(vec![IkkeMuligAaIdentifisereSisteFlytting])
            } else {
                let siste_flytting = flyttinger
                    .iter()
                    .filter(|f| f.dato.is_some())
                    .max_by(|f1, f2| f1.dato.cmp(&f2.dato))
                    .unwrap();
                return if siste_flytting.innflytting {
                    Ok(vec![SisteFlyttingVarInnTilNorge])
                } else {
                    Ok(vec![SisteFlyttingVarUtAvNorge])
                };
            }
        }
    }
}

#[derive(Debug, Clone)]
struct Flytting {
    pub innflytting: bool,
    pub dato: Option<NaiveDate>,
}

impl Flytting {
    fn fra_innflytting(innflytting: InnflyttingTilNorge) -> Flytting {
        let dato = innflytting
            .folkeregistermetadata
            .and_then(|metadata| metadata.ajourholdstidspunkt)
            .and_then(|t| parse_pdl_datetime(&t));
        Self {
            innflytting: true,
            dato,
        }
    }

    fn fra_utflytting(utflytting: UtflyttingFraNorge) -> Flytting {
        let dato = utflytting
            .utflyttingsdato
            .and_then(|t| NaiveDate::parse_from_str(&t, "%Y-%m-%d").ok());
        Self {
            innflytting: false,
            dato,
        }
    }
}

fn parse_pdl_datetime(s: &str) -> Option<NaiveDate> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.date_naive())
        .ok()
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
                .map(|dt| dt.date())
                .ok()
        })
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|dt| dt.date())
                .ok()
        })
}

#[cfg(test)]
mod tests {
    use crate::fakta::UtledeFakta;
    use crate::fakta::utflytting_fakta::UtledeUtflyttingFakta;
    use chrono::{Local, NaiveDate};
    use interne_hendelser::vo::Opplysning::{
        IkkeMuligAaIdentifisereSisteFlytting, IngenFlytteInformasjon, SisteFlyttingVarInnTilNorge,
        SisteFlyttingVarUtAvNorge,
    };
    use pdl_graphql::pdl::{
        InnflyttingTilNorge, InnflyttingTilNorgeFolkeregistermetadata, Person, UtflyttingFraNorge,
    };

    fn create_person(
        innflyttet: &Vec<Option<NaiveDate>>,
        utflyttet: &Vec<Option<NaiveDate>>,
    ) -> Person {
        let innflytting_til_norge = innflyttet
            .iter()
            .map(|t| {
                let tidspunkt = t
                    .and_then(|t| t.and_hms_opt(0, 0, 0))
                    .and_then(|t| Some(t.and_local_timezone(Local).unwrap()));
                InnflyttingTilNorge {
                    folkeregistermetadata: Some(InnflyttingTilNorgeFolkeregistermetadata {
                        gyldighetstidspunkt: None,
                        ajourholdstidspunkt: tidspunkt
                            .map(|t| t.format("%Y-%m-%dT%H:%M:%S").to_string()),
                    }),
                }
            })
            .collect::<Vec<InnflyttingTilNorge>>();
        let utflytting_fra_norge = utflyttet
            .iter()
            .map(|&dato| UtflyttingFraNorge {
                utflyttingsdato: dato.map(|d| d.format("%Y-%m-%d").to_string()),
                folkeregistermetadata: None,
            })
            .collect::<Vec<UtflyttingFraNorge>>();
        Person {
            innflytting_til_norge,
            utflytting_fra_norge,
            ..Default::default()
        }
    }

    #[test]
    fn ingen_inn_eller_utflyttinger_gir_ingen_flytte_info_fakta() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![];
        let utflyttet: Vec<Option<NaiveDate>> = vec![];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![IngenFlytteInformasjon]);
    }

    #[test]
    fn kun_innflyttinger_gir_innflytting_fakta() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![Some(Local::now().date_naive())];
        let utflyttet: Vec<Option<NaiveDate>> = vec![];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![SisteFlyttingVarInnTilNorge]);
    }

    #[test]
    fn kun_utflyttinger_gir_utflytting_fakta() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![];
        let utflyttet: Vec<Option<NaiveDate>> = vec![Some(Local::now().date_naive())];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![SisteFlyttingVarUtAvNorge]);
    }

    #[test]
    fn inn_og_utflyttinger_paa_samme_dato_gir_ikke_mulig_aa_vite_fakte() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![
            NaiveDate::from_ymd_opt(2026, 1, 13),
            NaiveDate::from_ymd_opt(2026, 1, 13),
        ];
        let utflyttet: Vec<Option<NaiveDate>> = vec![
            NaiveDate::from_ymd_opt(2026, 1, 13),
            NaiveDate::from_ymd_opt(2026, 1, 13),
        ];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![IkkeMuligAaIdentifisereSisteFlytting]);
    }

    #[test]
    fn inn_eller_utflyttinger_med_ukjent_dato_gir_ikke_mulig_aa_vite_fakte() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![NaiveDate::from_ymd_opt(2026, 3, 13), None];
        let utflyttet: Vec<Option<NaiveDate>> = vec![NaiveDate::from_ymd_opt(2026, 1, 13)];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![IkkeMuligAaIdentifisereSisteFlytting]);
    }

    #[test]
    fn innflytting_etter_utflytting_gir_innflyttet_fakte() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![NaiveDate::from_ymd_opt(2026, 3, 13)];
        let utflyttet: Vec<Option<NaiveDate>> = vec![NaiveDate::from_ymd_opt(2026, 1, 13)];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![SisteFlyttingVarInnTilNorge]);
    }

    #[test]
    fn utflytting_etter_innflytting_gir_utflyttet_fakte() {
        let innflyttet: Vec<Option<NaiveDate>> = vec![NaiveDate::from_ymd_opt(2026, 1, 13)];
        let utflyttet: Vec<Option<NaiveDate>> = vec![NaiveDate::from_ymd_opt(2026, 3, 13)];
        let person = create_person(&innflyttet, &utflyttet);
        let result = UtledeUtflyttingFakta::default().utlede_fakta(&person);
        let fakta = result.unwrap();
        assert_eq!(fakta, vec![SisteFlyttingVarUtAvNorge]);
    }
}
