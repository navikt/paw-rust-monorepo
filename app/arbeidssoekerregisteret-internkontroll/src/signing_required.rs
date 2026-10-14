use std::collections::HashMap;
use std::sync::Arc;

use chrono::DateTime;
use paw_kafka::topics::{Topic, get_topic};
use paw_rust_base::env::{RuntimeEnv, runtime_env};

/// Per topic: the stream time (ms) from which every record must be signed.
/// Topics without an entry have no signing requirement yet.
///
/// Set in code once the producers are verified to sign, so git tracks when
/// each requirement was turned on. A requirement is never removed.
#[derive(Debug, Clone)]
pub struct SigningRequiredFrom {
    by_topic: Arc<HashMap<String, i64>>,
}

impl SigningRequiredFrom {
    pub fn new(pairs: impl IntoIterator<Item = (impl Into<String>, i64)>) -> Self {
        let by_topic = pairs
            .into_iter()
            .map(|(topic, ms)| (topic.into(), ms))
            .collect();
        Self {
            by_topic: Arc::new(by_topic),
        }
    }

    pub fn for_runtime(runtime: &RuntimeEnv) -> Self {
        Self::new(
            required_from(runtime)
                .iter()
                .map(|(topic, at)| (get_topic(runtime, topic), rfc3339_ms(at))),
        )
    }

    /// The stream time (ms) from which records on `topic` must be signed.
    pub fn get(&self, topic: &str) -> Option<i64> {
        self.by_topic.get(topic).copied()
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
fn required_from(runtime: &RuntimeEnv) -> &'static [(Topic, &'static str)] {
    match runtime {
        RuntimeEnv::ProdGcp => &[],
        RuntimeEnv::DevGcp | RuntimeEnv::Local | RuntimeEnv::UnknownEnv(_) => &[],
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
        let required = SigningRequiredFrom::new([("topic-a", 1_000)]);
        assert_eq!(required.get("topic-a"), Some(1_000));
        assert_eq!(required.get("topic-b"), None);
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
