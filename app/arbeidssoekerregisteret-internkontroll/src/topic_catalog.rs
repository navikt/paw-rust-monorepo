use paw_kafka::{hwm::stream::topic_priority::TopicPriorityList, topics::Topic};
use paw_rust_base::env::RuntimeEnv;

pub fn subscribed_topics() -> Vec<Topic> {
    vec![
        Topic::Periode,
        Topic::Opplysninger,
        Topic::Profilering,
        Topic::PaaVegneAv,
        Topic::Bekreftelse,
        Topic::Hendelselogg,
        Topic::BekreftelseHendelseLogg,
        Topic::Egenvurdering,
    ]
}

pub fn topic_priorities(runtime_env: &RuntimeEnv) -> TopicPriorityList {
    TopicPriorityList::new(
        runtime_env,
        vec![
            (&Topic::Bekreftelse, 0),
            (&Topic::BekreftelseHendelseLogg, 5),
            (&Topic::Hendelselogg, 10),
            (&Topic::Periode, 50),
            (&Topic::Opplysninger, 60),
            (&Topic::Profilering, 70),
        ],
    )
}

pub const DEFAULT_SIGNING_REQUIRED_FROM: &str = "2026-10-01T00:00:00Z";

pub fn prod_signing_required() -> Vec<(Topic, &'static str)> {
    vec![
        // Add topics here when all its producers sign, for example
        // (Topic::Periode, "2026-10-12T00:00:00Z"),
        (Topic::Periode, "2026-10-01T00:00:00Z"),
        (Topic::Opplysninger, "2026-10-01T00:00:00Z"),
        (Topic::Profilering, "2026-10-01T00:00:00Z"),
        (Topic::PaaVegneAv, "2026-10-01T00:00:00Z"),
        (Topic::Bekreftelse, "2026-10-01T00:00:00Z"),
        (Topic::Hendelselogg, "2026-10-01T00:00:00Z"),
        (Topic::BekreftelseHendelseLogg, "2026-10-01T00:00:00Z"),
        (Topic::Egenvurdering, "2026-10-01T00:00:00Z"),
    ]
}

pub fn dev_signing_required() -> Vec<(Topic, &'static str)> {
    vec![
        // Add topics here when all its producers sign, for example
        // (Topic::Periode, "2026-10-12T00:00:00Z"),
        (Topic::Periode, "2026-10-01T00:00:00Z"),
        (Topic::Opplysninger, "2026-10-01T00:00:00Z"),
        (Topic::Profilering, "2026-10-01T00:00:00Z"),
        (Topic::PaaVegneAv, "2026-10-01T00:00:00Z"),
        (Topic::Bekreftelse, "2026-10-01T00:00:00Z"),
        (Topic::Hendelselogg, "2026-10-01T00:00:00Z"),
        (Topic::BekreftelseHendelseLogg, "2026-10-01T00:00:00Z"),
        (Topic::Egenvurdering, "2026-10-01T00:00:00Z"),
    ]
}
