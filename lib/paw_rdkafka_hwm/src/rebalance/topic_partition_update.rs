pub enum TopicPartitionUpdate {
    Assigned {
        topic_partition_hwms: Vec<(TopicPartition, i64)>,
    },
    Revoked {
        topic_partitions: Vec<TopicPartition>,
    },
    HiOffsetUpdate {
        topic_partition_offsets: Vec<(TopicPartition, KafkaOffsets)>,
    },
    InternalReceiverDisconnected,
    NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicPartition {
    pub topic: String,
    pub partition: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KafkaOffsets {
    pub hi_offset: i64,
    pub next_offset: i64,
}
