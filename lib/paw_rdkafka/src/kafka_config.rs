use rdkafka::config::RDKafkaLogLevel;
use rdkafka::ClientConfig;
use serde::Deserialize;
use serde_env_field::{env_field_wrap, EnvField};
use std::error::Error;
use std::time::SystemTime;

pub fn create_kafka_client_config(
    kafka_config: KafkaConfig,
) -> Result<ClientConfig, Box<dyn Error>> {
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
        .unwrap_or_else(|| EnvField::from(6000))
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

    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", kafka_config.brokers.into_inner())
        .set("group.id", group_id)
        .set("client.id", client_id)
        .set("session.timeout.ms", session_timeout_ms)
        .set("auto.offset.reset", auto_offset_reset)
        .set("enable.auto.commit", auto_commit)
        .set("security.protocol", security_protocol.clone())
        // Memory-constrained settings using only valid rdkafka properties
        // Note: fetch.max.bytes must be >= message.max.bytes (default 1MB)
        .set("message.max.bytes", "65536") // 64KB max message size
        .set("fetch.max.bytes", "131072") // 128KB max fetch (must be >= message.max.bytes)
        .set("fetch.message.max.bytes", "32768") // 32KB max per partition
        .set("queued.max.messages.kbytes", "1024") // 1MB internal queue size
        .set("queued.min.messages", "1") // Min messages in queue
        .set("socket.receive.buffer.bytes", "4096") // 4KB socket receive buffer
        .set("socket.send.buffer.bytes", "4096") // 4KB socket send buffer
        .set("fetch.min.bytes", "1") // Don't wait for much data
        .set("fetch.wait.max.ms", "100") // Don't wait long for data
        .set("receive.message.max.bytes", "200000") // 200KB max response (must be > fetch.max.bytes + 512)
        .set_log_level(RDKafkaLogLevel::Info);

    if security_protocol.clone().to_lowercase() == "ssl" {
        let private_key_path = kafka_config
            .private_key_path
            .ok_or_else(|| "Missing private key path".to_string())?;
        let certificate_path = kafka_config
            .certificate_path
            .ok_or_else(|| "Missing certificate path".to_string())?;
        let ca_path = kafka_config
            .ca_path
            .ok_or_else(|| "Missing ca path".to_string())?;
        config
            .set("ssl.key.location", private_key_path.into_inner())
            .set("ssl.certificate.location", certificate_path.into_inner())
            .set("ssl.ca.location", ca_path.into_inner());
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
            session_timeout_ms: Some(EnvField::from(6000)),
            hwm_version: EnvField::from(HWM_VERSION),
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
    pub fn rdkafka_client_config(&self) -> Result<ClientConfig, Box<dyn Error>> {
        create_kafka_client_config(self.clone())
    }
}

fn unix_timestamp_millis() -> Result<u128, Box<dyn Error>> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| e.into())
        .map(|d| d.as_millis())
}
