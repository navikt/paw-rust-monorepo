//! Default values for every tunable in [`KafkaConfig`](crate::kafka_config::KafkaConfig).
//!
//! These are the memory constrained values the apps ran on before 2026-09-21.
//! An app that needs more raises the field in its own TOML; nobody else pays
//! for it. `create_kafka_client_config` maps each one to its rdkafka property.

/// `hwm_version` when the app does not set one.
pub const HWM_VERSION: i16 = 1;

/// `enable.auto.commit`. Off: the HWM row in Postgres is the durable cursor,
/// Kafka consumer offsets are never used.
pub const AUTO_COMMIT: bool = false;

/// `session.timeout.ms`.
pub const SESSION_TIMEOUT_MS: i64 = 45000;

/// `statistics.interval.ms`.
pub const STATISTICS_INTERVAL_MS: i64 = 5000;

/// `auto.offset.reset` for a partition with no stored offset.
pub const AUTO_OFFSET_RESET: &str = "earliest";

/// `security.protocol`. Nais sets SSL through the app's TOML.
pub const SECURITY_PROTOCOL: &str = "PLAINTEXT";

/// `queued.min.messages`, per partition queue. Together with
/// [`QUEUED_MAX_MESSAGES_KBYTES`] this is the dominant memory term, since it
/// applies to every assigned partition across every subscribed topic.
pub const PARTITION_QUEUE_MIN_SIZE: i32 = 1;

/// `message.max.bytes`, the largest single message.
pub const MESSAGE_MAX_BYTES: i32 = 65536;

/// `fetch.max.bytes`. A cap for the whole fetch request, **not** per partition,
/// so an app subscribing to many co-partitioned topics divides this across all
/// of them and will want a higher value.
pub const FETCH_MAX_BYTES: i32 = 131072;

/// `max.partition.fetch.bytes`, the per partition share of a fetch response.
pub const MAX_PARTITION_FETCH_BYTES: i32 = 32768;

/// `receive.message.max.bytes`, a receive buffer per broker connection, so the
/// cost is multiplied by the broker count. librdkafka requires it to exceed
/// [`FETCH_MAX_BYTES`] + 512; raise both together.
pub const RECEIVE_MESSAGE_MAX_BYTES: i32 = 200000;

/// `queued.max.messages.kbytes`, the size cap on a partition queue.
pub const QUEUED_MAX_MESSAGES_KBYTES: i32 = 1024;

/// `socket.receive.buffer.bytes`. 0 would hand it to the OS default, which is
/// typically far larger.
pub const SOCKET_RECEIVE_BUFFER_BYTES: i32 = 4096;

/// `socket.send.buffer.bytes`.
pub const SOCKET_SEND_BUFFER_BYTES: i32 = 4096;

/// `fetch.min.bytes`. 1 means the broker answers as soon as it has anything.
pub const FETCH_MIN_BYTES: i32 = 1;

/// `fetch.wait.max.ms`, how long the broker holds a request waiting for
/// [`FETCH_MIN_BYTES`].
pub const FETCH_WAIT_MAX_MS: i32 = 100;

/// `fetch.queue.backoff.ms`, how long librdkafka postpones the next fetch for a
/// partition after that partition's queue hit [`PARTITION_QUEUE_MIN_SIZE`] or
/// [`QUEUED_MAX_MESSAGES_KBYTES`]. This is librdkafka's own default.
///
/// An app that merges several partitions on message timestamp wants this far
/// lower. A full second of not refetching is long enough for a high traffic
/// partition to drain, and a partition with an empty queue cannot take part in
/// the merge.
pub const FETCH_QUEUE_BACKOFF_MS: i32 = 1000;

/// librdkafka log level. Note that librdkafka log events land on the same queue
/// the consumer polls, so a chatty level costs poll iterations.
pub const LOG_LEVEL: &str = "info";
