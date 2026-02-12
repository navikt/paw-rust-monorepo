use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    ErOver18Aar,
    #[serde(alias = "ER_UNDER_18_AAR")]
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
    #[serde(alias = "BOSATT_ETTER_FREG_LOVEN")]
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

#[test]
fn test_deserialize_opplysning_med_alias() {
    let json_data = r#"
        ["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN", "UKJENT_OPPLYSNING"]
    "#;

    // Deserialiser til Vec<Opplysning>
    let opplysninger: Vec<Opplysning> = serde_json::from_str(json_data).unwrap();

    // Forvent at de to første verdiene blir korrekt deserialisert
    assert!(matches!(opplysninger[0], Opplysning::ErUnder18Aar));
    assert!(matches!(opplysninger[1], Opplysning::BosattEtterFregLoven));
    // Den tredje verdien har ingen alias, så den blir UkjentOpplysning
    assert!(matches!(opplysninger[2], Opplysning::UkjentOpplysning));
}