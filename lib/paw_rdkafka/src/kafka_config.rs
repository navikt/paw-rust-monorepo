use paw_rust_base::env_var::get_env;
use rdkafka::ClientConfig;
use rdkafka::config::RDKafkaLogLevel;
use std::error::Error;
use std::time::SystemTime;

pub fn create_kafka_client_config(
    kafka_config: KafkaConfig,
) -> Result<ClientConfig, Box<dyn Error>> {
    let brokers = get_env("KAFKA_BROKERS")?;
    let kafka_private_key_path = get_env("KAFKA_PRIVATE_KEY_PATH")?;
    let kafka_certificate_path = get_env("KAFKA_CERTIFICATE_PATH")?;
    let kafka_ca_path = get_env("KAFKA_CA_PATH")?;
    let auto_commit = if kafka_config.auto_commit {
        "true"
    } else {
        "false"
    };
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", brokers)
        .set("group.id", kafka_config.group_id)
        .set("client.id", kafka_config.client_id)
        .set(
            "session.timeout.ms",
            kafka_config.session_timeout_ms.to_string(),
        )
        .set(
            "auto.offset.reset",
            kafka_config.auto_offset_reset,
        )
        .set("enable.auto.commit", auto_commit)
        .set(
            "security.protocol",
            kafka_config.security_protocol,
        )
        .set("ssl.key.location", kafka_private_key_path)
        .set("ssl.certificate.location", kafka_certificate_path)
        .set("ssl.ca.location", kafka_ca_path)
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
    Ok(config)
}

#[derive(Debug, Clone)]
pub struct KafkaConfig {
    pub group_id: String,
    pub client_id: String,
    pub auto_commit: bool,
    pub security_protocol: String,
    pub auto_offset_reset: String,
    pub session_timeout_ms: i64,
    pub hwm_version: i16,
}

const HWM_VERSION: i16 = 1;

impl Default for KafkaConfig {
    fn default() -> Self {
        let client_id = format!(
            "client-{}",
            unix_timestamp_millis().expect("Failed to get unix timestamp millis")
        );
        Self {
            group_id: "default-group".to_string(),
            client_id: client_id,
            auto_commit: false,
            security_protocol: "ssl".to_string(),
            auto_offset_reset: "earliest".to_string(),
            session_timeout_ms: 6000,
            hwm_version: HWM_VERSION,
        }
    }
}

impl KafkaConfig {
    pub fn new(group_id: &str, security_protocol: &str) -> Self {
        KafkaConfig {
            group_id: group_id.to_string(),
            security_protocol: security_protocol.to_string(),
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
