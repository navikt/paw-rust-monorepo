use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Opplysning {
    ForhaandsgodkjentAvAnsatt,
    SammeSomInnloggetBruker,
    IkkeSammeSomInnloggerBruker,
    AnsattIkkeTilgang,
    AnsattTilgang,
    IkkeAnsatt,
    SystemIkkeTilgang,
    SystemTilgang,
    IkkeSystem,
    #[strum(serialize = "ER_OVER_18_AAR")]
    #[serde(rename = "ER_OVER_18_AAR")]
    ErOver18Aar,
    #[strum(serialize = "ER_UNDER_18_AAR")]
    #[serde(rename = "ER_UNDER_18_AAR")]
    ErUnder18Aar,
    UkjentFoedselsdato,
    UkjentFoedselsaar,
    TokenxPidIkkeFunnet,
    OpphoertIdentitet,
    IkkeBosatt,
    Doed,
    Savnet,
    HarNorskAdresse,
    HarUtenlandskAdresse,
    HarRegistrertAdresseIEuEoes,
    IngenAdresseFunnet,
    BosattEtterFregLoven,
    Dnummer,
    UkjentForenkletFregStatus,
    HarGyldigOppholdstillatelse,
    OppholdstilatellseUtgaatt,
    BarnFoedtINorgeUtenOppholdstillatelse,
    IngenInformasjonOmOppholdstillatelse,
    UkjentStatusForOppholdstillatelse,
    PersonIkkeFunnet,
    SisteFlyttingVarUtAvNorge,
    SisteFlyttingVarInnTilNorge,
    IkkeMuligAaIdentifisereSisteFlytting,
    IngenFlytteInformasjon,
    ErEuEoesStatsborger,
    ErGbrStatsborger,
    ErNorskStatsborger,
    ErFeilretting,
    UgyldigFeilretting,
    #[serde(other)]
    UkjentOpplysning,
}

#[cfg(test)]
mod tests {
    use crate::vo::Opplysning;
    use std::str::FromStr;

    #[test]
    fn test_json_serialize() {
        let expected_json_data = r#"["ER_OVER_18_AAR","ER_UNDER_18_AAR","BOSATT_ETTER_FREG_LOVEN","BARN_FOEDT_I_NORGE_UTEN_OPPHOLDSTILLATELSE","UKJENT_OPPLYSNING"]"#;

        let data = vec![
            Opplysning::ErOver18Aar,
            Opplysning::ErUnder18Aar,
            Opplysning::BosattEtterFregLoven,
            Opplysning::BarnFoedtINorgeUtenOppholdstillatelse,
            Opplysning::UkjentOpplysning,
        ];

        let json_data = serde_json::to_string(&data).unwrap();

        assert_eq!(json_data, expected_json_data);
    }

    #[test]
    fn test_json_deserialize() {
        let json_data = r#"
        [
            "ER_OVER_18_AAR",
            "ER_UNDER_18_AAR",
            "BOSATT_ETTER_FREG_LOVEN",
            "BARN_FOEDT_I_NORGE_UTEN_OPPHOLDSTILLATELSE",
            "UKJENT_OPPLYSNING"
        ]
        "#
        .chars()
        .filter(|&c| !c.is_whitespace())
        .collect::<String>();

        // Deserialiser til Vec<Opplysning>
        let opplysninger: Vec<Opplysning> = serde_json::from_str(json_data.as_str()).unwrap();

        // Forvent at de to første verdiene blir korrekt deserialisert
        assert!(matches!(opplysninger[0], Opplysning::ErOver18Aar));
        assert!(matches!(opplysninger[1], Opplysning::ErUnder18Aar));
        assert!(matches!(opplysninger[2], Opplysning::BosattEtterFregLoven));
        assert!(matches!(
            opplysninger[3],
            Opplysning::BarnFoedtINorgeUtenOppholdstillatelse
        ));
        // Den tredje verdien har ingen alias, så den blir UkjentOpplysning
        assert!(matches!(opplysninger[4], Opplysning::UkjentOpplysning));
    }

    #[test]
    fn test_strum_to_string() {
        assert_eq!("ER_OVER_18_AAR", Opplysning::ErOver18Aar.to_string());
        assert_eq!("ER_UNDER_18_AAR", Opplysning::ErUnder18Aar.to_string());
        assert_eq!(
            "BOSATT_ETTER_FREG_LOVEN",
            Opplysning::BosattEtterFregLoven.to_string()
        );
        assert_eq!(
            "BARN_FOEDT_I_NORGE_UTEN_OPPHOLDSTILLATELSE",
            Opplysning::BarnFoedtINorgeUtenOppholdstillatelse.to_string()
        );
        assert_eq!(
            "UKJENT_OPPLYSNING",
            Opplysning::UkjentOpplysning.to_string()
        );
    }

    #[test]
    fn test_strum_from_string() {
        assert_eq!(
            Opplysning::from_str("ER_OVER_18_AAR").unwrap(),
            Opplysning::ErOver18Aar
        );
        assert_eq!(
            Opplysning::from_str("ER_UNDER_18_AAR").unwrap(),
            Opplysning::ErUnder18Aar
        );
        assert_eq!(
            Opplysning::from_str("BOSATT_ETTER_FREG_LOVEN").unwrap(),
            Opplysning::BosattEtterFregLoven
        );
        assert_eq!(
            Opplysning::from_str("BARN_FOEDT_I_NORGE_UTEN_OPPHOLDSTILLATELSE").unwrap(),
            Opplysning::BarnFoedtINorgeUtenOppholdstillatelse
        );
        assert_eq!(
            Opplysning::from_str("UKJENT_OPPLYSNING").unwrap(),
            Opplysning::UkjentOpplysning
        );
    }
}
