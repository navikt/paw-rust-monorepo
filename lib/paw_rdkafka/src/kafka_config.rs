use crate::defaults;
use crate::error::KafkaError;
use rdkafka::ClientConfig;
use rdkafka::config::RDKafkaLogLevel;
use serde::Deserialize;
use serde_env_field::{EnvField, env_field_wrap};
use std::time::{SystemTime, SystemTimeError};

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
    pub statistics_interval_ms: Option<i64>,
    pub hwm_version: i16,
    pub partition_assignment_strategy: Option<String>,
    pub partition_queue_min_size: Option<i32>,
    pub message_max_bytes: Option<i32>,
    pub fetch_max_bytes: Option<i32>,
    pub max_partition_fetch_bytes: Option<i32>,
    pub receive_message_max_bytes: Option<i32>,
    pub queued_max_messages_kbytes: Option<i32>,
    pub socket_receive_buffer_bytes: Option<i32>,
    pub socket_send_buffer_bytes: Option<i32>,
    pub fetch_min_bytes: Option<i32>,
    pub fetch_wait_max_ms: Option<i32>,
    pub fetch_queue_backoff_ms: Option<i32>,
    pub log_level: Option<String>,
}

impl Default for KafkaConfig {
    /// Every tunable is `None`, meaning "use the value in [`defaults`]".
    /// `create_kafka_client_config` is the only place a default is applied,
    /// so there is one list of numbers rather than two that can drift.
    fn default() -> Self {
        Self {
            brokers: EnvField::from("localhost:9092".to_string()),
            group_id_prefix: EnvField::from("default-group-id-prefix".to_string()),
            hwm_version: EnvField::from(defaults::HWM_VERSION),
            auto_commit: None,
            security_protocol: None,
            private_key_path: None,
            certificate_path: None,
            ca_path: None,
            auto_offset_reset: None,
            session_timeout_ms: None,
            statistics_interval_ms: None,
            partition_assignment_strategy: None,
            partition_queue_min_size: None,
            message_max_bytes: None,
            fetch_max_bytes: None,
            max_partition_fetch_bytes: None,
            receive_message_max_bytes: None,
            queued_max_messages_kbytes: None,
            socket_receive_buffer_bytes: None,
            socket_send_buffer_bytes: None,
            fetch_min_bytes: None,
            fetch_wait_max_ms: None,
            fetch_queue_backoff_ms: None,
            log_level: None,
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

pub fn create_kafka_client_config(kafka_config: KafkaConfig) -> Result<ClientConfig, KafkaError> {
    let hwm_version = kafka_config.hwm_version.into_inner();
    let client_nonce = unix_timestamp_millis().expect("Failed to get unix timestamp millis");
    let group_id_prefix = kafka_config.group_id_prefix.into_inner();
    let group_id = format!("{}-v{}", group_id_prefix, hwm_version);
    let client_id = format!("{}-client-{}", group_id_prefix, client_nonce);
    let auto_commit = value_or(kafka_config.auto_commit, defaults::AUTO_COMMIT);
    let session_timeout_ms = value_or(
        kafka_config.session_timeout_ms,
        defaults::SESSION_TIMEOUT_MS,
    );
    let statistics_interval_ms = value_or(
        kafka_config.statistics_interval_ms,
        defaults::STATISTICS_INTERVAL_MS,
    );
    let auto_offset_reset = value_or(
        kafka_config.auto_offset_reset,
        defaults::AUTO_OFFSET_RESET.to_string(),
    );
    let security_protocol = value_or(
        kafka_config.security_protocol,
        defaults::SECURITY_PROTOCOL.to_string(),
    );
    let partition_assignment_strategy = kafka_config
        .partition_assignment_strategy
        .map(|s| s.into_inner());
    let partition_queue_min_size = value_or(
        kafka_config.partition_queue_min_size,
        defaults::PARTITION_QUEUE_MIN_SIZE,
    );
    let message_max_bytes = value_or(kafka_config.message_max_bytes, defaults::MESSAGE_MAX_BYTES);
    let fetch_max_bytes = value_or(kafka_config.fetch_max_bytes, defaults::FETCH_MAX_BYTES);
    let max_partition_fetch_bytes = value_or(
        kafka_config.max_partition_fetch_bytes,
        defaults::MAX_PARTITION_FETCH_BYTES,
    );
    let receive_message_max_bytes = value_or(
        kafka_config.receive_message_max_bytes,
        defaults::RECEIVE_MESSAGE_MAX_BYTES,
    );
    let queued_max_messages_kbytes = value_or(
        kafka_config.queued_max_messages_kbytes,
        defaults::QUEUED_MAX_MESSAGES_KBYTES,
    );
    let socket_receive_buffer_bytes = value_or(
        kafka_config.socket_receive_buffer_bytes,
        defaults::SOCKET_RECEIVE_BUFFER_BYTES,
    );
    let socket_send_buffer_bytes = value_or(
        kafka_config.socket_send_buffer_bytes,
        defaults::SOCKET_SEND_BUFFER_BYTES,
    );
    let fetch_min_bytes = value_or(kafka_config.fetch_min_bytes, defaults::FETCH_MIN_BYTES);
    let fetch_wait_max_ms = value_or(kafka_config.fetch_wait_max_ms, defaults::FETCH_WAIT_MAX_MS);
    let fetch_queue_backoff_ms = value_or(
        kafka_config.fetch_queue_backoff_ms,
        defaults::FETCH_QUEUE_BACKOFF_MS,
    );
    let log_level = value_or(kafka_config.log_level, defaults::LOG_LEVEL.to_string());

    // librdkafka rejects the client unless this holds
    if receive_message_max_bytes <= fetch_max_bytes + 512 {
        return Err(KafkaError::Config(format!(
            "receive_message_max_bytes ({receive_message_max_bytes}) must be greater than fetch_max_bytes ({fetch_max_bytes}) + 512"
        )));
    }

    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", kafka_config.brokers.into_inner())
        .set("group.id", group_id)
        .set("client.id", client_id)
        .set("session.timeout.ms", session_timeout_ms.to_string())
        .set("statistics.interval.ms", statistics_interval_ms.to_string())
        .set("auto.offset.reset", auto_offset_reset)
        .set("enable.auto.commit", auto_commit.to_string())
        .set("security.protocol", security_protocol.clone())
        .set("message.max.bytes", message_max_bytes.to_string())
        .set("fetch.max.bytes", fetch_max_bytes.to_string())
        .set(
            "max.partition.fetch.bytes",
            max_partition_fetch_bytes.to_string(),
        )
        .set(
            "receive.message.max.bytes",
            receive_message_max_bytes.to_string(),
        )
        .set("queued.min.messages", partition_queue_min_size.to_string())
        .set(
            "queued.max.messages.kbytes",
            queued_max_messages_kbytes.to_string(),
        )
        .set(
            "socket.receive.buffer.bytes",
            socket_receive_buffer_bytes.to_string(),
        )
        .set(
            "socket.send.buffer.bytes",
            socket_send_buffer_bytes.to_string(),
        )
        .set("fetch.min.bytes", fetch_min_bytes.to_string())
        .set("fetch.wait.max.ms", fetch_wait_max_ms.to_string())
        .set("fetch.queue.backoff.ms", fetch_queue_backoff_ms.to_string())
        .set_log_level(parse_log_level(&log_level)?);

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

fn value_or<T>(field: Option<EnvField<T>>, default: T) -> T {
    field.map(EnvField::into_inner).unwrap_or(default)
}

fn parse_log_level(value: &str) -> Result<RDKafkaLogLevel, KafkaError> {
    match value.to_lowercase().as_str() {
        "emerg" => Ok(RDKafkaLogLevel::Emerg),
        "alert" => Ok(RDKafkaLogLevel::Alert),
        "critical" => Ok(RDKafkaLogLevel::Critical),
        "error" => Ok(RDKafkaLogLevel::Error),
        "warning" => Ok(RDKafkaLogLevel::Warning),
        "notice" => Ok(RDKafkaLogLevel::Notice),
        "info" => Ok(RDKafkaLogLevel::Info),
        "debug" => Ok(RDKafkaLogLevel::Debug),
        other => Err(KafkaError::Config(format!(
            "Unknown kafka log level: {other}"
        ))),
    }
}

fn unix_timestamp_millis() -> Result<u128, SystemTimeError> {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_memory_constrained_values() {
        let config = KafkaConfig::new("test", "PLAINTEXT")
            .rdkafka_client_config()
            .expect("config should build");

        assert_eq!(config.get("message.max.bytes"), Some("65536"));
        assert_eq!(config.get("fetch.max.bytes"), Some("131072"));
        assert_eq!(config.get("max.partition.fetch.bytes"), Some("32768"));
        assert_eq!(config.get("receive.message.max.bytes"), Some("200000"));
        assert_eq!(config.get("queued.min.messages"), Some("1"));
        assert_eq!(config.get("queued.max.messages.kbytes"), Some("1024"));
        assert_eq!(config.get("socket.receive.buffer.bytes"), Some("4096"));
        assert_eq!(config.get("socket.send.buffer.bytes"), Some("4096"));
        assert_eq!(config.get("fetch.min.bytes"), Some("1"));
        assert_eq!(config.get("fetch.wait.max.ms"), Some("100"));
        assert_eq!(config.get("fetch.queue.backoff.ms"), Some("1000"));
        assert_eq!(config.get("session.timeout.ms"), Some("45000"));
        assert_eq!(config.get("statistics.interval.ms"), Some("5000"));
        assert_eq!(config.get("auto.offset.reset"), Some("earliest"));
        assert_eq!(config.get("enable.auto.commit"), Some("false"));
    }

    #[test]
    fn overrides_reach_the_client_config() {
        let config = KafkaConfig {
            message_max_bytes: Some(EnvField::from(1000000)),
            fetch_max_bytes: Some(EnvField::from(1048576)),
            receive_message_max_bytes: Some(EnvField::from(1500000)),
            partition_queue_min_size: Some(EnvField::from(1000)),
            statistics_interval_ms: Some(EnvField::from(1000)),
            ..KafkaConfig::new("test", "PLAINTEXT")
        }
        .rdkafka_client_config()
        .expect("config should build");

        assert_eq!(config.get("message.max.bytes"), Some("1000000"));
        assert_eq!(config.get("fetch.max.bytes"), Some("1048576"));
        assert_eq!(config.get("receive.message.max.bytes"), Some("1500000"));
        assert_eq!(config.get("queued.min.messages"), Some("1000"));
        assert_eq!(config.get("statistics.interval.ms"), Some("1000"));
    }

    #[test]
    fn rejects_receive_buffer_smaller_than_fetch_max() {
        let result = KafkaConfig {
            fetch_max_bytes: Some(EnvField::from(1048576)),
            ..KafkaConfig::new("test", "PLAINTEXT")
        }
        .rdkafka_client_config();

        assert!(matches!(result, Err(KafkaError::Config(_))));
    }

    #[test]
    fn rejects_unknown_log_level() {
        let result = KafkaConfig {
            log_level: Some(EnvField::from("verbose".to_string())),
            ..KafkaConfig::new("test", "PLAINTEXT")
        }
        .rdkafka_client_config();

        assert!(matches!(result, Err(KafkaError::Config(_))));
    }

    #[test]
    fn accepts_known_log_levels() {
        for level in [
            "emerg", "alert", "critical", "error", "warning", "notice", "info", "debug", "DEBUG",
        ] {
            let result = KafkaConfig {
                log_level: Some(EnvField::from(level.to_string())),
                ..KafkaConfig::new("test", "PLAINTEXT")
            }
            .rdkafka_client_config();

            assert!(result.is_ok(), "log level {level} should be accepted");
        }
    }
}
