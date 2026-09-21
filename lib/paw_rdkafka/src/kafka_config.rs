use crate::error::KafkaError;
use rdkafka::ClientConfig;
use rdkafka::config::RDKafkaLogLevel;
use serde::Deserialize;
use serde_env_field::{EnvField, env_field_wrap};
use std::time::{SystemTime, SystemTimeError};

pub fn create_kafka_client_config(kafka_config: KafkaConfig) -> Result<ClientConfig, KafkaError> {
    let hwm_version = kafka_config.hwm_version.into_inner();
    let client_nonce = unix_timestamp_millis().expect("Failed to get unix timestamp millis");
    let group_id_prefix = kafka_config.group_id_prefix.into_inner();
    let group_id = format!("{}-v{}", group_id_prefix, hwm_version);
    let client_id = format!("{}-client-{}", group_id_prefix, client_nonce);
    let auto_commit = kafka_config
        .auto_commit
        .unwrap_or_else(|| EnvField::from(false))
        .into_inner()
        .to_string();
    let session_timeout_ms = kafka_config
        .session_timeout_ms
        .unwrap_or_else(|| EnvField::from(45000))
        .into_inner()
        .to_string();
    let auto_offset_reset = kafka_config
        .auto_offset_reset
        .unwrap_or_else(|| EnvField::from("earliest".to_string()))
        .into_inner();
    let security_protocol = kafka_config
        .security_protocol
        .unwrap_or_else(|| EnvField::from("PLAINTEXT".to_string()))
        .into_inner();
    let partition_assignment_strategy = kafka_config
        .partition_assignment_strategy
        .map(|s| s.into_inner());
    let partition_queue_min_size = kafka_config
        .partition_queue_min_size
        .unwrap_or_else(|| EnvField::from(1))
        .into_inner()
        .to_string();
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", kafka_config.brokers.into_inner())
        .set("group.id", group_id)
        .set("client.id", client_id)
        .set("session.timeout.ms", session_timeout_ms)
        .set("auto.offset.reset", auto_offset_reset)
        .set("enable.auto.commit", auto_commit)
        .set("security.protocol", security_protocol.clone())
        // 1. Message size safety (Default 1MB) - prevents client crashes on large records
        .set("message.max.bytes", "1000000")
        // 2. Local partition queue caps (Controls buffer depth for timestamp alignment)
        .set("queued.min.messages", partition_queue_min_size) // 1000
        .set("queued.max.messages.kbytes", "1024") // 1MB per partition queue
        // 3. Fetch sizes (Keep per-partition chunks small, total fetch payload large)
        .set("max.partition.fetch.bytes", "65536") // 64KB per partition per fetch
        .set("fetch.max.bytes", "1048576") // 1MB global fetch limit across partitions
        // 4. Transport payload cap (MUST be > fetch.max.bytes + 512)
        .set("receive.message.max.bytes", "1500000") // 1.5MB to safely hold the 1MB fetch payload
        // 5. Low latency fetch timing
        .set("fetch.min.bytes", "1") // Do not wait on broker for aggregation
        .set("fetch.wait.max.ms", "100") // 100ms max broker delay
        .set_log_level(RDKafkaLogLevel::Info);

    if security_protocol.clone().to_lowercase() == "ssl" {
        let private_key_path = kafka_config
            .private_key_path
            .ok_or_else(|| "Missing private key path".to_string())
            .map_err(KafkaError::Config)?;
        let certificate_path = kafka_config
            .certificate_path
            .ok_or_else(|| "Missing certificate path".to_string())
            .map_err(KafkaError::Config)?;
        let ca_path = kafka_config
            .ca_path
            .ok_or_else(|| "Missing ca path".to_string())
            .map_err(KafkaError::Config)?;
        config
            .set("ssl.key.location", private_key_path.into_inner())
            .set("ssl.certificate.location", certificate_path.into_inner())
            .set("ssl.ca.location", ca_path.into_inner());
    }

    // Opt-in: kun satt dersom appens config eksplisitt spesifiserer det (f.eks. kartlegging-api,
    // som er avhengig av en deterministisk "range"-assignor for å garantere at co-partisjonerte
    // topics tildeles samme gruppemedlem per partisjonsnummer). Andre apper som deler denne
    // klient-konfigurasjonen er upåvirket med mindre de også setter feltet.
    if let Some(strategy) = partition_assignment_strategy {
        config.set("partition.assignment.strategy", strategy);
    }

    Ok(config)
}

#[env_field_wrap]
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    pub group_id_prefix: String,
    pub auto_commit: Option<bool>,
    pub security_protocol: Option<String>,
    pub private_key_path: Option<String>,
    pub certificate_path: Option<String>,
    pub ca_path: Option<String>,
    pub auto_offset_reset: Option<String>,
    pub session_timeout_ms: Option<i64>,
    pub hwm_version: i16,
    pub partition_assignment_strategy: Option<String>,
    pub partition_queue_min_size: Option<i32>,
}

const HWM_VERSION: i16 = 1;

impl Default for KafkaConfig {
    fn default() -> Self {
        Self {
            brokers: EnvField::from("localhost:9092".to_string()),
            group_id_prefix: EnvField::from("default-group-id-prefix".to_string()),
            auto_commit: Some(EnvField::from(false)),
            security_protocol: Some(EnvField::from("PLAINTEXT".to_string())),
            private_key_path: None,
            certificate_path: None,
            ca_path: None,
            auto_offset_reset: Some(EnvField::from("earliest".to_string())),
            session_timeout_ms: Some(EnvField::from(45000)),
            hwm_version: EnvField::from(HWM_VERSION),
            partition_assignment_strategy: None,
            partition_queue_min_size: Some(EnvField::from(1)),
        }
    }
}

impl KafkaConfig {
    pub fn new(group_id_prefix: &str, security_protocol: &str) -> Self {
        KafkaConfig {
            group_id_prefix: EnvField::from(group_id_prefix.to_string()),
            security_protocol: Some(EnvField::from(security_protocol.to_string())),
            ..Default::default()
        }
    }
    pub fn rdkafka_client_config(&self) -> Result<ClientConfig, KafkaError> {
        create_kafka_client_config(self.clone())
    }
}

fn unix_timestamp_millis() -> Result<u128, SystemTimeError> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
}
