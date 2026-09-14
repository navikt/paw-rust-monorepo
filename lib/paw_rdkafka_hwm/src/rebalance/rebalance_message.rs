pub enum RebalanceMessage {
    Assigned {
        topic_partitions: Vec<TopicPartition>,
    },
    Revoked {
        topic_partitions: Vec<TopicPartition>,
    },
    InternalReceiverDisconnected,
    NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TopicPartition {
    pub topic: String,
    pub partition: i32,
}
