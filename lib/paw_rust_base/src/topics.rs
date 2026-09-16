use crate::env::RuntimeEnv;

pub enum Topic {
    Periode,
    Opplysninger,
    Profilering,
    PaaVegneAv,
    Bekreftelse,
    Hendelselogg,
    BekreftelseHendelseLogg,
    Egenvurdering,
}

pub fn get_topic_names(runtime: &RuntimeEnv, topics: &[Topic]) -> Vec<&'static str> {
    topics
        .iter()
        .map(|topic| get_topic(runtime, topic))
        .collect()
}

pub fn get_topic(runtime: &RuntimeEnv, topic: &Topic) -> &'static str {
    match topic {
        Topic::Periode => "paw.arbeidssokerperioder-v1",
        Topic::Opplysninger => "paw.opplysninger-om-arbeidssoeker-v1",
        Topic::Profilering => "paw.arbeidssoker-profilering-v1",
        Topic::PaaVegneAv => match runtime {
            RuntimeEnv::ProdGcp => "paw.arbeidssoker-bekreftelse-paavegneav-v2",
            RuntimeEnv::DevGcp => "paw.arbeidssoker-bekreftelse-paavegneav-v1",
            RuntimeEnv::Local => "paw.arbeidssoker-bekreftelse-paavegneav-v1",
            RuntimeEnv::UnknownEnv(_) => "paw.arbeidssoker-bekreftelse-paavegneav-v1",
        },
        Topic::Bekreftelse => "paw.arbeidssoker-bekreftelse-v1",
        Topic::Hendelselogg => "paw.arbeidssoker-hendelseslogg-v1",
        Topic::BekreftelseHendelseLogg => "paw.arbeidssoker-bekreftelse-hendelseslogg-v1",
        Topic::Egenvurdering => "paw.arbeidssoeker-egenvurdering-v1",
    }
}
