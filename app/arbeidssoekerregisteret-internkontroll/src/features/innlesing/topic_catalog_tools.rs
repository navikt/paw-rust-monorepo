use std::collections::HashMap;

use paw_kafka::{
    hwm::stream::topic_priority::TopicPriorityList,
    topics::{Topic, get_topic},
};
use paw_rust_base::env::RuntimeEnv;

///2026-10-01T00:00:00Z as unixtimestamp in milllis
const DEFAULT_SIGNING_REQUIRED_FROM_TIMESTAMP_MS: i64 = 1767312000000;

pub struct SigningReqiredFromTimestampMS(i64);

pub struct TopicConfig {
    pub topic: Topic,
    pub backup_required: bool,
    priority: Option<i64>,
    dev_signing_required_from: SigningReqiredFromTimestampMS,
    prod_signing_required_from: SigningReqiredFromTimestampMS,
}

impl TopicConfig {
    pub fn priority(&self) -> i64 {
        self.priority.unwrap_or(100)
    }

    pub fn signing_required_from(
        &self,
        runtime_env: &RuntimeEnv,
    ) -> &SigningReqiredFromTimestampMS {
        match runtime_env {
            RuntimeEnv::ProdGcp => &self.prod_signing_required_from,
            RuntimeEnv::DevGcp => &self.dev_signing_required_from,
            RuntimeEnv::Local => &self.dev_signing_required_from,
            RuntimeEnv::UnknownEnv(_) => &self.dev_signing_required_from,
        }
    }

    pub fn new(
        topic: Topic,
        backup_required: bool,
        priority: i64,
        dev_signing_required_from: SigningReqiredFromTimestampMS,
        prod_signing_required_from: SigningReqiredFromTimestampMS,
    ) -> Self {
        Self {
            topic,
            backup_required,
            priority: Some(priority),
            dev_signing_required_from,
            prod_signing_required_from,
        }
    }

    pub fn with_defaults(topic: Topic, backup_required: bool) -> Self {
        Self {
            topic,
            backup_required,
            priority: None,
            dev_signing_required_from: SigningReqiredFromTimestampMS(
                DEFAULT_SIGNING_REQUIRED_FROM_TIMESTAMP_MS,
            ),
            prod_signing_required_from: SigningReqiredFromTimestampMS(
                DEFAULT_SIGNING_REQUIRED_FROM_TIMESTAMP_MS,
            ),
        }
    }

    pub fn with_priority(self, priority: i64) -> Self {
        Self {
            priority: Some(priority),
            ..self
        }
    }

    /// Example timestamp "2026-10-01T00:00:00Z";
    #[allow(dead_code, reason = "Bare defaults are i bruk")]
    pub fn with_dev_signing_required_from(self, time: &'static str) -> Self {
        let timestamp_ms = time
            .parse::<chrono::DateTime<chrono::Utc>>()
            .expect("Invalid timestamp format")
            .timestamp_millis();
        Self {
            dev_signing_required_from: SigningReqiredFromTimestampMS(timestamp_ms),
            ..self
        }
    }

    /// Example timestamp "2026-10-01T00:00:00Z";
    #[allow(dead_code, reason = "Bare defaults are i bruk")]
    pub fn with_prod_signing_required_from(self, time: &'static str) -> Self {
        let timestamp_ms = time
            .parse::<chrono::DateTime<chrono::Utc>>()
            .expect("Invalid timestamp format")
            .timestamp_millis();
        Self {
            prod_signing_required_from: SigningReqiredFromTimestampMS(timestamp_ms),
            ..self
        }
    }
}

pub struct SigningRequiredCatalog {
    map: HashMap<String, i64>,
}

impl SigningRequiredCatalog {
    pub fn new(runtime_env: &RuntimeEnv, configs: &[TopicConfig]) -> Self {
        let map: HashMap<String, i64> = configs
            .iter()
            .map(|tc| {
                let topic_name = get_topic(runtime_env, &tc.topic).to_string();
                let signing_req_from = tc.signing_required_from(runtime_env).0;
                (topic_name, signing_req_from)
            })
            .collect();
        Self { map }
    }

    pub fn signing_required(&self, topic: &str, reference_time: i64) -> Option<bool> {
        self.map
            .get(topic)
            .map(|req_from| reference_time >= *req_from)
    }
}

pub struct TopicCatalog {
    runtime_env: RuntimeEnv,
    pub topics: Vec<TopicConfig>,
    topic_names: HashMap<Topic, String>,
    topic_backup: HashMap<String, bool>,
}

impl TopicCatalog {
    pub fn new(runtime_env: RuntimeEnv, topics: Vec<TopicConfig>) -> Self {
        let topic_names = topics
            .iter()
            .map(|tc| {
                (
                    tc.topic.clone(),
                    get_topic(&runtime_env, &tc.topic).to_string(),
                )
            })
            .collect();
        let topic_backup = topics
            .iter()
            .map(|tc| {
                (
                    get_topic(&runtime_env, &tc.topic).to_string(),
                    tc.backup_required,
                )
            })
            .collect();
        Self {
            runtime_env,
            topics,
            topic_names,
            topic_backup,
        }
    }

    pub fn get_topic_priority_list(&self) -> TopicPriorityList {
        let topics_with_priorities = self
            .topics
            .iter()
            .filter_map(|topic_config| {
                topic_config
                    .priority
                    .map(|priority| (&topic_config.topic, priority))
            })
            .collect();
        TopicPriorityList::new(&self.runtime_env, topics_with_priorities)
    }

    pub fn signing_required_catalog(&self) -> SigningRequiredCatalog {
        SigningRequiredCatalog::new(&self.runtime_env, &self.topics)
    }

    pub fn topic_names(&self) -> Vec<String> {
        self.topic_names.values().cloned().collect()
    }

    pub fn topic_names_mapping(&self) -> Vec<(&String, &Topic)> {
        self.topic_names.iter().map(|kv| (kv.1, kv.0)).collect()
    }

    pub fn backup_required(&self, topic_name: &str) -> Option<bool> {
        self.topic_backup.get(topic_name).copied()
    }
}
