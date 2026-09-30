use std::time::Duration;

use crate::stream::topic_priority::TopicPriorityList;

/// Settings for `PawKafkaConsumerStream`.
pub struct PawKafkaStreamConfig {
    /// Maximum time `load` waits for the partition queues to fill before it
    /// gives up and selects among the messages already buffered.
    pub max_idle: Duration,
    /// Soft limit for the number of messages buffered per partition queue.
    pub internal_buffer_size: usize,
    /// HWM version used when filtering messages from the main consumer queue.
    pub hwm_version: i16,
    /// Tie-breaker between topics when head timestamps are equal.
    pub topic_priorities: TopicPriorityList,
    /// Minimum age of a message before it is returned while another queue for
    /// the same partition is empty. Messages are returned at once when every
    /// queue for the partition has a head message.
    pub grace: Duration,
}
