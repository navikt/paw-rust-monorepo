use interne_hendelser::vo::Opplysning;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Betingelse {
    Har(Opplysning),
    HarIkke(Opplysning),
    ErNorskEllerTredjelandsborger,
}

impl Betingelse {
    pub fn eval(&self, opplysninger: &[Opplysning]) -> bool {
        match self {
            Betingelse::Har(o) => opplysninger.contains(o),
            Betingelse::HarIkke(o) => !opplysninger.contains(o),
            Betingelse::ErNorskEllerTredjelandsborger => {
                opplysninger.contains(&Opplysning::ErNorskStatsborger)
                    || !opplysninger.contains(&Opplysning::ErEuEoesStatsborger)
            }
        }
    }
}
