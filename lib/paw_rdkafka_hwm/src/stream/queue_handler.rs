use std::collections::VecDeque;
use std::future::Future;
use std::sync::LazyLock;

use futures::FutureExt;
use paw_rdkafka::error::KafkaError;
use prometheus::{Gauge, GaugeVec, register_gauge_vec};
use rdkafka::{Message, Timestamp};

use crate::stream::{message_wrapper::MessageWrapper, paw_kafka_stream::StreamError};

use crate::rebalance::hwm_rebalance_handler::HwmRebalanceHandler;

use rdkafka::consumer::stream_consumer::StreamPartitionQueue;

use rdkafka::message::OwnedMessage;

use crate::rebalance::topic_partition_update::{KafkaOffsets, TopicPartition};

pub trait PartitionMessageSource: Send {
    fn recv(&self) -> impl Future<Output = Result<OwnedMessage, StreamError>> + Send;
}

impl PartitionMessageSource for StreamPartitionQueue<HwmRebalanceHandler> {
    async fn recv(&self) -> Result<OwnedMessage, StreamError> {
        StreamPartitionQueue::recv(self)
            .await
            .map(|message| message.detach())
            .map_err(|error| {
                tracing::error!(%error, "Failed to receive message from rdkafka stream");
                StreamError::FailedToReadRecord(error.to_string())
            })
    }
}

pub type KafkaQueueHandler = QueueHandler<StreamPartitionQueue<HwmRebalanceHandler>>;

pub struct QueueHandler<S: PartitionMessageSource> {
    pub key: TopicPartition,
    head: VecDeque<MessageWrapper>,
    message_source: S,
    internal_buffer_size: usize,
    last_timestamp_gauge: Gauge,
    next_timestamp_gauge: Gauge,
    depth_gauge: Gauge,
    lag_gauge: Gauge,
    offsets: Option<KafkaOffsets>,
    current_offset: i64,
    current_timestamp: Option<i64>,
}

impl<S: PartitionMessageSource> QueueHandler<S> {
    pub fn new(
        key: TopicPartition,
        message_source: S,
        internal_buffer_size: usize,
        current_offset: i64,
    ) -> Self {
        let topic = key.topic.clone();
        let partition = key.partition.to_string();
        let last_timestamp_gauge =
            LAST_QUEUE_HANDLER_TIMESTAMP.with_label_values(&[&topic, &partition]);
        let next_timestamp_gauge =
            NEXT_QUEUE_HANDLER_TIMESTAMP.with_label_values(&[&topic, &partition]);
        let depth_gauge = QUEUE_HANDLER_DEPTH.with_label_values(&[&topic, &partition]);
        let lag_gauge = QUEUE_HANDLER_LAG.with_label_values(&[&topic, &partition]);
        // 0 would render as 1970; NaN renders as a gap.
        last_timestamp_gauge.set(f64::NAN);
        next_timestamp_gauge.set(f64::NAN);
        depth_gauge.set(0.0);
        // Lag is unknown until the first HiOffsetUpdate arrives.
        lag_gauge.set(f64::NAN);
        Self {
            key,
            head: VecDeque::new(),
            message_source,
            internal_buffer_size,
            last_timestamp_gauge,
            next_timestamp_gauge,
            depth_gauge,
            lag_gauge,
            offsets: None,
            current_offset,
            current_timestamp: None,
        }
    }

    fn update_lag_gauge(&self) {
        let Some(offsets) = &self.offsets else {
            return;
        };
        let not_yet_fetched = (offsets.hi_offset - offsets.next_offset).max(0);
        let lag = not_yet_fetched + offsets.message_queue_count + self.head.len() as i64;
        self.lag_gauge.set(lag as f64);
    }

    pub fn key(&self) -> &TopicPartition {
        &self.key
    }

    pub fn take_head(&mut self) -> Option<MessageWrapper> {
        let wrapped_msg = self.head.pop_front()?;
        self.last_timestamp_gauge
            .set(timestamp_ms(Some(&wrapped_msg.message)));
        self.next_timestamp_gauge
            .set(timestamp_ms(self.head.front().map(|w| &w.message)));
        self.depth_gauge.set(self.head.len() as f64);
        self.update_lag_gauge();
        Some(wrapped_msg)
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
        if self.head.len() > (self.internal_buffer_size / 4) {
            return Ok(());
        }
        while self.is_lagging() && self.head.len() < self.internal_buffer_size {
            self.push(self.message_source.recv().await?)?;
        }
        if self.head.len() < (self.internal_buffer_size / 4) {
            match self.message_source.recv().now_or_never() {
                Some(Ok(msg)) => {
                    self.push(msg)?;
                }
                Some(Err(err)) => {
                    return Err(StreamError::FailedToReadRecord(err.to_string()));
                }
                None => {}
            }
        }
        Ok(())
    }

    fn is_lagging(&self) -> bool {
        self.offsets.as_ref().is_none_or(|offsets| {
            offsets.next_offset < offsets.hi_offset || offsets.message_queue_count > 0
        })
    }

    pub fn has_stalled(&self) -> bool {
        self.is_empty() && self.is_lagging()
    }

    pub fn set_offsets(&mut self, offsets: KafkaOffsets) {
        self.offsets = Some(offsets);
        self.update_lag_gauge();
    }

    fn push(&mut self, msg: OwnedMessage) -> Result<(), StreamError> {
        if self.current_offset >= msg.offset() {
            return Err(StreamError::MessageOutOfSequence {
                topic: self.key.topic.clone(),
                partition: self.key.partition,
                current_offset: self.current_offset,
                message_offset: msg.offset(),
            });
        }
        if self.head.is_empty() {
            self.next_timestamp_gauge.set(timestamp_ms(Some(&msg)));
        }
        self.current_offset = msg.offset();
        let message_timestamp = msg.timestamp().to_millis();
        let current_timestamp = self.current_timestamp;
        let delta = match (current_timestamp, message_timestamp) {
            (_, None) => {
                self.current_timestamp = None;
                None
            }
            (None, Some(msg_ts)) => {
                self.current_timestamp = Some(msg_ts);
                None
            }
            (Some(current_ts), Some(msg_ts)) => {
                self.current_timestamp = Some(msg_ts);
                let delta = msg_ts - current_ts;
                if delta < 0 {
                    tracing::warn!(
                        kafka.topic = self.key.topic,
                        kafka.partition = self.key.partition,
                        previous_offset = self.current_offset,
                        message_offset = msg.offset(),
                        previous_timestamp_ms = current_timestamp,
                        message_timestamp_ms = message_timestamp,
                        back_in_time_ms = delta.abs(),
                        "kafka.partition_timestamp_out_of_sequence"
                    );
                }
                Some(delta)
            }
        };
        self.head.push_back(MessageWrapper::new(msg, delta));
        self.depth_gauge.set(self.head.len() as f64);
        self.update_lag_gauge();
        Ok(())
    }

    pub fn add_message(&mut self, msg: OwnedMessage) -> Result<usize, StreamError> {
        let topic = msg.topic().to_string();
        let partition = msg.partition();
        if self.key.topic != topic || self.key.partition != partition {
            return Err(KafkaError::UnexpectedMessage(format!(
                "Message topic/partition ({}/{}) does not match queue key ({}/{})",
                topic, partition, self.key.topic, self.key.partition
            ))
            .into());
        }
        self.push(msg)?;
        Ok(self.head.len())
    }

    pub fn timestamp(&self) -> Option<Timestamp> {
        self.head.front().map(|msg| msg.message.timestamp())
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_empty()
    }
}

impl<S: PartitionMessageSource> Drop for QueueHandler<S> {
    fn drop(&mut self) {
        let topic = self.key.topic.clone();
        let partition = self.key.partition.to_string();
        let _ = LAST_QUEUE_HANDLER_TIMESTAMP.remove_label_values(&[&topic, &partition]);
        let _ = NEXT_QUEUE_HANDLER_TIMESTAMP.remove_label_values(&[&topic, &partition]);
        let _ = QUEUE_HANDLER_DEPTH.remove_label_values(&[&topic, &partition]);
        let _ = QUEUE_HANDLER_LAG.remove_label_values(&[&topic, &partition]);
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

static QUEUE_HANDLER_LAG: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_stream_queue_handler_lag",
        "Total number of messages not yet delivered to the application for this partition: \
         not-yet-fetched-from-broker (hi_offset - next_offset), plus buffered in rdkafka's \
         internal queue (message_queue_count), plus buffered in the queue handler's own head",
        &["topic", "partition"]
    )
    .expect("Failed to create gauge")
});
