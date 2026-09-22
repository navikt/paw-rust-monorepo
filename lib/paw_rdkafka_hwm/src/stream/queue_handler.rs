use std::collections::VecDeque;
use std::sync::LazyLock;

use paw_rdkafka::error::KafkaError;
use prometheus::{Gauge, GaugeVec, register_gauge_vec};
use rdkafka::{Message, Timestamp};
use tracing::Span;

use crate::stream::paw_kafka_stream::StreamError;

use crate::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;

use rdkafka::consumer::stream_consumer::StreamPartitionQueue;

use rdkafka::message::OwnedMessage;

use crate::rebalance::rebalance_message::TopicPartition;

pub struct QueueHandler {
    pub key: TopicPartition,
    head: VecDeque<OwnedMessage>,
    rdkafka_stream: StreamPartitionQueue<HwmRebalanceHandler>,
    internal_buffer_size: usize,
    last_timestamp_gauge: Gauge,
    next_timestamp_gauge: Gauge,
}

impl QueueHandler {
    pub fn new(
        key: TopicPartition,
        rdkafka_stream: StreamPartitionQueue<HwmRebalanceHandler>,
        internal_buffer_size: usize,
    ) -> Self {
        let topic = key.topic.clone();
        let partition = key.partition.to_string();
        Self {
            key,
            head: VecDeque::new(),
            rdkafka_stream,
            internal_buffer_size,
            last_timestamp_gauge: LAST_QUEUE_HANDLER_TIMESTAMP
                .with_label_values(&[&topic, &partition]),
            next_timestamp_gauge: NEXT_QUEUE_HANDLER_TIMESTAMP
                .with_label_values(&[&topic, &partition]),
        }
    }

    pub fn key(&self) -> &TopicPartition {
        &self.key
    }

    pub fn take_head(&mut self) -> Option<OwnedMessage> {
        let msg = self.head.pop_front()?;
        if let Some(ts) = msg.timestamp().to_millis() {
            self.last_timestamp_gauge.set(ts as f64);
        }
        let next_ts = self
            .head
            .front()
            .and_then(|msg| msg.timestamp().to_millis());
        self.next_timestamp_gauge.set(next_ts.unwrap_or(0) as f64);
        Some(msg)
    }
    #[tracing::instrument(
        skip(self),
        name = "paw_kafka_stream.queue_update",
        fields(
            topic = self.key.topic.as_str(),
            partition = self.key.partition,
            current_queue_size = self.head.len() as u64)
        )]
    pub async fn update(&mut self) -> Result<(), StreamError> {
        if self.head.is_empty() {
            while self.head.len() < self.internal_buffer_size {
                match self.rdkafka_stream.recv().await {
                    Ok(record) => {
                        let record = record.detach();
                        if self.head.is_empty() {
                            self.next_timestamp_gauge
                                .set(record.timestamp().to_millis().unwrap_or(0) as f64);
                        }
                        self.head.push_back(record);
                        Span::current().record("record_added", self.head.len() as u64);
                    }
                    Err(e) => return Err(StreamError::FailedToReadRecord(e.to_string())),
                }
            }
        }
        Ok(())
    }

    pub fn add_message(&mut self, msg: OwnedMessage) -> Result<usize, KafkaError> {
        let topic = msg.topic().to_string();
        let partition = msg.partition();
        if self.key.topic != topic || self.key.partition != partition {
            return Err(KafkaError::UnexpectedMessage(format!(
                "Message topic/partition ({}/{}) does not match queue key ({}/{})",
                topic, partition, self.key.topic, self.key.partition
            )));
        }
        self.head.push_back(msg);
        Ok(self.head.len())
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        self.head.front().map(|msg| msg.timestamp())
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_empty()
    }
}

impl Drop for QueueHandler {
    fn drop(&mut self) {
        let topic = self.key.topic.clone();
        let partition = self.key.partition.to_string();
        let _ = LAST_QUEUE_HANDLER_TIMESTAMP.remove_label_values(&[&topic, &partition]);
        let _ = NEXT_QUEUE_HANDLER_TIMESTAMP.remove_label_values(&[&topic, &partition]);
    }
}

static LAST_QUEUE_HANDLER_TIMESTAMP: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_stream_queue_handler_next_timestamp",
        "The timestamp of the last message retrieved from the queue handler",
        &["topic", "partition"]
    )
    .expect("Failed to create gauge")
});
static NEXT_QUEUE_HANDLER_TIMESTAMP: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_stream_queue_handler_next_timestamp",
        "The timestamp of the next message retrieved from the queue handler",
        &["topic", "partition"]
    )
    .expect("Failed to create gauge")
});
