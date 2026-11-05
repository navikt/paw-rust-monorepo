use paw_kafka::topics::Topic;
use paw_rust_base::env::RuntimeEnv;

use crate::features::innlesing::topic_catalog_tools::{TopicCatalog, TopicConfig};

/// Example timestamp "2026-10-01T00:00:00Z";
pub fn subscribed_topics(runtime_env: RuntimeEnv) -> TopicCatalog {
    TopicCatalog::new(
        runtime_env,
        vec![
            TopicConfig::with_defaults(Topic::Periode, false).with_priority(50),
            TopicConfig::with_defaults(Topic::Opplysninger, false).with_priority(60),
            TopicConfig::with_defaults(Topic::Profilering, false).with_priority(70),
            TopicConfig::with_defaults(Topic::Bekreftelse, true).with_priority(0),
            TopicConfig::with_defaults(Topic::Hendelselogg, true).with_priority(10),
            TopicConfig::with_defaults(Topic::BekreftelseHendelseLogg, false).with_priority(5),
            TopicConfig::with_defaults(Topic::Egenvurdering, true),
            TopicConfig::with_defaults(Topic::PaaVegneAv, true),
        ],
    )
}
