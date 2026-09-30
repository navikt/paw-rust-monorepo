use std::time::Duration;

use paw_rdkafka::error::KafkaError;
use paw_rdkafka::kafka_config::KafkaConfig;

use crate::stream::topic_priority::TopicPriorityList;

pub const DEFAULT_MAX_IDLE: Duration = Duration::from_millis(500);
pub const DEFAULT_INTERNAL_BUFFER_SIZE: usize = 200;
pub const DEFAULT_MAIN_CONSUMER_NONE_TRESHOLD: usize = 10;
/// Added on top of `statistics.interval.ms + fetch.wait.max.ms` when
/// `from_kafka_config` computes `grace`. Covers network round trips and clock
/// skew, which do not grow with the intervals, so the margin is fixed.
pub const GRACE_MARGIN: Duration = Duration::from_millis(400);

/// Settings for `PawKafkaConsumerStream`.
pub struct PawKafkaStreamConfig {
    /// Maximum time `load` waits for the partition queues to fill before it
    /// gives up and selects among the messages already buffered.
    pub max_idle: Duration,
    /// Soft limit for the number of messages buffered per partition queue.
    pub internal_buffer_size: usize,
    /// HWM version used when filtering messages from the main consumer queue.
    pub hwm_version: i16,
    /// Number of empty polls of the main consumer queue (with no rebalance
    /// events) before `drain_and_rebalance` stops polling.
    pub main_consumer_none_treshold: usize,
    /// Tie-breaker between topics when head timestamps are equal.
    pub topic_priorities: TopicPriorityList,
    /// Minimum age of a message before it is returned while another queue for
    /// the same partition is empty. Messages are returned at once when every
    /// queue for the partition has a head message.
    ///
    /// An empty queue counts as caught up based on the latest rdkafka
    /// statistics snapshot, and `hi_offset` in that snapshot is only as fresh
    /// as the last fetch response. Two settings make sense:
    ///
    /// - `Duration::ZERO`: lowest latency. Messages from a queue that looked
    ///   caught up but was not show up later as backward timestamp jumps
    ///   (`back_in_time`).
    /// - `statistics.interval.ms + fetch.wait.max.ms + margin`: timestamp
    ///   order across topics on the same partition. The consumer config
    ///   decides the latency cost; lowering those two settings lowers the
    ///   grace needed. `from_kafka_config` computes this value.
    ///
    /// A value in between removes some jumps but guarantees nothing.
    ///
    /// The order is only as good as the timestamps. With `CreateTime`, a
    /// message delayed at the producer (linger, retries) longer than `grace`
    /// can still arrive out of order; `LogAppendTime` on the topics avoids
    /// that. Clock skew between broker and consumer also eats into the margin.
    pub grace: Duration,
}

impl PawKafkaStreamConfig {
    /// Defaults for a stream that keeps timestamp order across topics on the
    /// same partition. `hwm_version` and `grace` come from `kafka`; `grace` is
    /// `statistics.interval.ms + fetch.wait.max.ms + GRACE_MARGIN`.
    ///
    /// Override single fields with struct update syntax, for example
    /// `grace: Duration::ZERO` for lowest latency:
    ///
    /// ```ignore
    /// PawKafkaStreamConfig {
    ///     grace: Duration::ZERO,
    ///     ..PawKafkaStreamConfig::from_kafka_config(&kafka_config, priorities)?
    /// }
    /// ```
    ///
    /// Fails when statistics are disabled (`statistics.interval.ms = 0`).
    /// The stream decides whether a partition queue is lagging from the
    /// statistics, and without them every partition waits forever.
    pub fn from_kafka_config(
        kafka: &KafkaConfig,
        topic_priorities: TopicPriorityList,
    ) -> Result<Self, KafkaError> {
        let statistics_interval = kafka.statistics_interval();
        if statistics_interval.is_zero() {
            return Err(KafkaError::Config(
                "statistics_interval_ms must be greater than 0: the stream needs rdkafka \
                 statistics to tell whether a partition is caught up"
                    .to_string(),
            ));
        }
        Ok(Self {
            max_idle: DEFAULT_MAX_IDLE,
            internal_buffer_size: DEFAULT_INTERNAL_BUFFER_SIZE,
            hwm_version: *kafka.hwm_version,
            main_consumer_none_treshold: DEFAULT_MAIN_CONSUMER_NONE_TRESHOLD,
            topic_priorities,
            grace: statistics_interval + kafka.fetch_wait_max() + GRACE_MARGIN,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_env_field::EnvField;

    #[test]
    fn grace_er_statistikk_pluss_fetch_wait_pluss_margin() {
        let kafka = KafkaConfig {
            statistics_interval_ms: Some(EnvField::from(1000)),
            fetch_wait_max_ms: Some(EnvField::from(100)),
            hwm_version: EnvField::from(20),
            ..KafkaConfig::new("test", "PLAINTEXT")
        };

        let config =
            PawKafkaStreamConfig::from_kafka_config(&kafka, TopicPriorityList::empty()).unwrap();

        assert_eq!(
            config.grace,
            Duration::from_millis(1000 + 100) + GRACE_MARGIN
        );
        assert_eq!(config.hwm_version, 20);
    }

    #[test]
    fn grace_bruker_kafka_defaults_naar_ikke_satt() {
        let kafka = KafkaConfig::new("test", "PLAINTEXT");

        let config =
            PawKafkaStreamConfig::from_kafka_config(&kafka, TopicPriorityList::empty()).unwrap();

        assert_eq!(
            config.grace,
            kafka.statistics_interval() + kafka.fetch_wait_max() + GRACE_MARGIN
        );
    }

    #[test]
    fn avviser_avskrudd_statistikk() {
        let kafka = KafkaConfig {
            statistics_interval_ms: Some(EnvField::from(0)),
            ..KafkaConfig::new("test", "PLAINTEXT")
        };

        let result = PawKafkaStreamConfig::from_kafka_config(&kafka, TopicPriorityList::empty());

        assert!(matches!(result, Err(KafkaError::Config(_))));
    }
}
