use crate::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;
use crate::rebalance::topic_partition_update::{TopicPartition, TopicPartitionUpdate};
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
                            partition_statistics.hi_offset,
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
