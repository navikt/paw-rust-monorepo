use std::collections::HashMap;
use std::sync::Arc;

use chrono::DateTime;
use paw_kafka::topics::{Topic, get_topic};
use paw_rust_base::env::{RuntimeEnv, runtime_env};

use crate::topic_catalog::{
    DEFAULT_SIGNING_REQUIRED_FROM, dev_signing_required, prod_signing_required,
};

/// Per topic: the stream time (ms) from which every record must be signed.
/// Topics without an entry have no signing requirement yet.
///
/// Set in code once the producers are verified to sign, so git tracks when
/// each requirement was turned on. A requirement is never removed.
#[derive(Debug, Clone)]
pub struct SigningRequiredFrom {
    by_topic: Arc<HashMap<String, i64>>,
    default_signing_required_from: i64,
}

impl SigningRequiredFrom {
    pub fn new(default_at: i64, pairs: impl IntoIterator<Item = (impl Into<String>, i64)>) -> Self {
        let by_topic = pairs
            .into_iter()
            .map(|(topic, ms)| (topic.into(), ms))
            .collect();
        Self {
            by_topic: Arc::new(by_topic),
            default_signing_required_from: default_at,
        }
    }

    pub fn for_runtime(runtime: &RuntimeEnv) -> Self {
        Self::new(
            rfc3339_ms(DEFAULT_SIGNING_REQUIRED_FROM),
            required_from(runtime)
                .iter()
                .map(|(topic, at)| (get_topic(runtime, topic), rfc3339_ms(at))),
        )
    }

    /// The stream time (ms) from which records on `topic` must be signed.
    pub fn get(&self, topic: &str) -> i64 {
        self.by_topic
            .get(topic)
            .copied()
            .unwrap_or(self.default_signing_required_from)
    }
}

impl Default for SigningRequiredFrom {
    /// Uses the runtime environment the app runs in.
    fn default() -> Self {
        Self::for_runtime(&runtime_env())
    }
}

/// Add a topic here when all its producers sign, for example
/// `(Topic::Periode, "2026-10-12T00:00:00Z")`.
fn required_from(runtime: &RuntimeEnv) -> Vec<(Topic, &'static str)> {
    match runtime {
        RuntimeEnv::ProdGcp => prod_signing_required(),
        RuntimeEnv::DevGcp | RuntimeEnv::Local | RuntimeEnv::UnknownEnv(_) => {
            dev_signing_required()
        }
    }
}

fn rfc3339_ms(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value)
        .unwrap_or_else(|e| panic!("invalid signing requirement time {value}: {e}"))
        .timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slaar_opp_grense_per_topic() {
        let default_at = 123_i64;
        let required = SigningRequiredFrom::new(default_at, [("topic-a", 1_000)]);
        assert_eq!(required.get("topic-a"), 1_000);
        assert_eq!(required.get("topic-b"), default_at);
    }

    #[test]
    fn grensene_for_alle_miljoer_kan_leses() {
        for runtime in [RuntimeEnv::ProdGcp, RuntimeEnv::DevGcp, RuntimeEnv::Local] {
            SigningRequiredFrom::for_runtime(&runtime);
        }
    }

    #[test]
    fn tidspunkt_leses_som_millisekunder() {
        assert_eq!(rfc3339_ms("1970-01-01T00:00:01Z"), 1_000);
    }
}
