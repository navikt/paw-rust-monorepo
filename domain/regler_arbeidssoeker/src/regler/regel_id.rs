use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RegelId {
    IkkeFunnet,
    Savnet,
    Doed,
    Opphoert,
    Under18Aar,
    IkkeBosattINorgeIHenholdTilFolkeregisterloven,
    ForhaandsgodkjentAvAnsatt,
    Over18AarOgBosattEtterFregLoven,
    UkjentAlder,
    EuEoesStatsborgerOver18Aar,
    ErStatsborgerILandMedAvtale,
    EuEoesStatsborgerMenHarStatusIkkeBosatt,
}

impl fmt::Display for RegelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RegelId::IkkeFunnet => "Person ikke funnet",
            RegelId::Savnet => "Er registrert som savnet",
            RegelId::Doed => "Er registrert som død",
            RegelId::Opphoert => "Har ugyldig/annullert identitet",
            RegelId::Under18Aar => "Er under 18 år",
            RegelId::IkkeBosattINorgeIHenholdTilFolkeregisterloven => {
                "Avvist fordi personen ikke er bosatt i Norge i henhold til folkeregisterloven"
            }
            RegelId::ForhaandsgodkjentAvAnsatt => "Er forhåndsgodkjent av ansatt",
            RegelId::Over18AarOgBosattEtterFregLoven => {
                "Er over 18 år, er bosatt i Norge i henhold Folkeregisterloven"
            }
            RegelId::UkjentAlder => "Kunne ikke fastslå alder",
            RegelId::EuEoesStatsborgerOver18Aar => "Er EU/EØS statsborger",
            RegelId::ErStatsborgerILandMedAvtale => "Er statsborger i land med avtale",
            RegelId::EuEoesStatsborgerMenHarStatusIkkeBosatt => {
                "Er EU/EØS statsborger, men har status 'ikke bosatt'"
            }
        })
    }
}
