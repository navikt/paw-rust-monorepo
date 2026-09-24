use crate::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use crate::rebalance::topic_partition_update::{
    KafkaOffsets, TopicPartition, TopicPartitionUpdate,
};
use rdkafka::{ClientContext, Statistics};

impl ClientContext for HwmRebalanceHandler {
    fn stats(&self, statistics: Statistics) {
        let Some(sender) = &self.sender else {
            return;
        };

        let topic_partition_offsets = statistics
            .topics
            .into_iter()
            .flat_map(|(topic, topic_statistics)| {
                topic_statistics.partitions.into_iter().map(
                    move |(partition, partition_statistics)| {
                        (
                            TopicPartition {
                                topic: topic.clone(),
                                partition,
                            },
                            KafkaOffsets {
                                hi_offset: partition_statistics.hi_offset,
                                next_offset: partition_statistics.next_offset,
                                message_queue_count: partition_statistics.msgq_cnt,
                            },
                        )
                    },
                )
            })
            .collect();

        sender
            .send(TopicPartitionUpdate::HiOffsetUpdate {
                topic_partition_offsets,
            })
            .unwrap_or_else(|error| {
                tracing::warn!(%error, "Failed to send high offset update message");
            });
    }
}
