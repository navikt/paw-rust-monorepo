use std::collections::VecDeque;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use futures::FutureExt;
use paw_rdkafka::error::KafkaError;
use prometheus::{Gauge, GaugeVec, register_gauge_vec};
use rdkafka::{Message, Timestamp};

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
    depth_gauge: Gauge,
    empty_since: Option<Instant>,
}

impl QueueHandler {
    pub fn new(
        key: TopicPartition,
        rdkafka_stream: StreamPartitionQueue<HwmRebalanceHandler>,
        internal_buffer_size: usize,
    ) -> Self {
        let topic = key.topic.clone();
        let partition = key.partition.to_string();
        let last_timestamp_gauge =
            LAST_QUEUE_HANDLER_TIMESTAMP.with_label_values(&[&topic, &partition]);
        let next_timestamp_gauge =
            NEXT_QUEUE_HANDLER_TIMESTAMP.with_label_values(&[&topic, &partition]);
        let depth_gauge = QUEUE_HANDLER_DEPTH.with_label_values(&[&topic, &partition]);
        // 0 would render as 1970; NaN renders as a gap.
        last_timestamp_gauge.set(f64::NAN);
        next_timestamp_gauge.set(f64::NAN);
        depth_gauge.set(0.0);
        Self {
            key,
            head: VecDeque::new(),
            rdkafka_stream,
            internal_buffer_size,
            last_timestamp_gauge,
            next_timestamp_gauge,
            depth_gauge,
            empty_since: Some(Instant::now()),
        }
    }

    pub fn key(&self) -> &TopicPartition {
        &self.key
    }

    pub fn take_head(&mut self) -> Option<OwnedMessage> {
        let msg = self.head.pop_front()?;
        self.last_timestamp_gauge.set(timestamp_ms(Some(&msg)));
        self.next_timestamp_gauge
            .set(timestamp_ms(self.head.front()));
        self.depth_gauge.set(self.head.len() as f64);
        if self.head.is_empty() {
            self.empty_since = Some(Instant::now());
        }
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
    pub async fn update(&mut self, max_idle: Duration) -> Result<(), StreamError> {
        if self.empty_for().is_some_and(|idle_for| idle_for < max_idle) {
            match self.rdkafka_stream.recv().await {
                Ok(record) => self.push(record.detach()),
                Err(e) => return Err(StreamError::FailedToReadRecord(e.to_string())),
            }
        }
        while self.head.len() < self.internal_buffer_size {
            match self.rdkafka_stream.recv().now_or_never() {
                Some(Ok(record)) => self.push(record.detach()),
                Some(Err(e)) => return Err(StreamError::FailedToReadRecord(e.to_string())),
                None => break,
            }
        }
        Ok(())
    }

    fn push(&mut self, msg: OwnedMessage) {
        if self.head.is_empty() {
            self.next_timestamp_gauge.set(timestamp_ms(Some(&msg)));
            self.empty_since = None;
        }
        self.head.push_back(msg);
        self.depth_gauge.set(self.head.len() as f64);
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
        self.push(msg);
        Ok(self.head.len())
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        self.head.front().map(|msg| msg.timestamp())
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_empty()
    }

    pub fn empty_for(&self) -> Option<Duration> {
        self.is_empty()
            .then(|| self.empty_since.map(|t| t.elapsed()))
            .flatten()
    }
}

impl Drop for QueueHandler {
    fn drop(&mut self) {
        let topic = self.key.topic.clone();
        let partition = self.key.partition.to_string();
        let _ = LAST_QUEUE_HANDLER_TIMESTAMP.remove_label_values(&[&topic, &partition]);
        let _ = NEXT_QUEUE_HANDLER_TIMESTAMP.remove_label_values(&[&topic, &partition]);
        let _ = QUEUE_HANDLER_DEPTH.remove_label_values(&[&topic, &partition]);
    }
}

fn timestamp_ms(msg: Option<&OwnedMessage>) -> f64 {
    msg.and_then(|msg| msg.timestamp().to_millis())
        .map_or(f64::NAN, |ts| ts as f64)
}

static LAST_QUEUE_HANDLER_TIMESTAMP: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_stream_queue_handler_last_timestamp",
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

static QUEUE_HANDLER_DEPTH: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_stream_queue_handler_depth",
        "Number of messages currently buffered in the queue handler",
        &["topic", "partition"]
    )
    .expect("Failed to create gauge")
});
