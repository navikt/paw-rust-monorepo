use std::collections::{HashMap, VecDeque};
use std::future::{Future, pending};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use paw_rdkafka_hwm::rebalance::topic_partition_update::{
    KafkaOffsets, TopicPartition, TopicPartitionUpdate,
};
use paw_rdkafka_hwm::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use paw_rdkafka_hwm::stream::queue_handler::PartitionMessageSource;
use paw_rdkafka_hwm::stream::stream_wrapper::{ConsumerMessageSource, PawKafkaConsumerStream};
use rdkafka::Message;
use rdkafka::message::{OwnedMessage, Timestamp};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

#[tokio::test]
async fn leverer_laveste_timestamp_fra_tildelte_partisjoner() {
    let topic_a = TopicPartition {
        topic: "topic-a".to_string(),
        partition: 0,
    };
    let topic_b = TopicPartition {
        topic: "topic-b".to_string(),
        partition: 0,
    };
    let consumer = FakeConsumer::new([
        (
            topic_a.clone(),
            FakePartitionSource::new([message("topic-a", 0, 0, 200)]),
        ),
        (
            topic_b.clone(),
            FakePartitionSource::new([message("topic-b", 0, 0, 100)]),
        ),
    ]);
    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: vec![(topic_a.clone(), -1), (topic_b.clone(), -1)],
        })
        .unwrap();
    sender
        .send(TopicPartitionUpdate::HiOffsetUpdate {
            topic_partition_offsets: vec![offsets(topic_a, 1), offsets(topic_b, 1)],
        })
        .unwrap();

    let stream = PawKafkaConsumerStream::new(
        receiver,
        consumer,
        Duration::from_millis(10),
        10,
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        1,
        1,
    );

    let (stream, first) = stream.receive().await.unwrap();
    let (_, second) = stream.receive().await.unwrap();

    let first = first.unwrap();
    assert_eq!(first.topic(), "topic-b");
    assert_eq!(first.timestamp(), Timestamp::CreateTime(100));

    let second = second.unwrap();
    assert_eq!(second.topic(), "topic-a");
    assert_eq!(second.timestamp(), Timestamp::CreateTime(200));
}

fn offsets(topic_partition: TopicPartition, hi_offset: i64) -> (TopicPartition, KafkaOffsets) {
    (
        topic_partition,
        KafkaOffsets {
            hi_offset,
            next_offset: hi_offset,
        },
    )
}

#[derive(Clone)]
struct FakePartitionSource {
    messages: Arc<Mutex<VecDeque<OwnedMessage>>>,
}

impl FakePartitionSource {
    fn new(messages: impl IntoIterator<Item = OwnedMessage>) -> Self {
        Self {
            messages: Arc::new(Mutex::new(messages.into_iter().collect())),
        }
    }
}

impl PartitionMessageSource for FakePartitionSource {
    async fn recv(&self) -> Result<OwnedMessage, StreamError> {
        if let Some(message) = self.messages.lock().unwrap().pop_front() {
            return Ok(message);
        }
        pending().await
    }
}

struct FakeConsumer {
    partition_sources: Mutex<HashMap<TopicPartition, FakePartitionSource>>,
}

impl FakeConsumer {
    fn new(
        partition_sources: impl IntoIterator<Item = (TopicPartition, FakePartitionSource)>,
    ) -> Self {
        Self {
            partition_sources: Mutex::new(partition_sources.into_iter().collect()),
        }
    }
}

impl ConsumerMessageSource for FakeConsumer {
    type PartitionSource = FakePartitionSource;

    fn recv(&self) -> impl Future<Output = Result<OwnedMessage, StreamError>> + Send {
        pending()
    }

    fn split_partition_queue(
        self: &Arc<Self>,
        topic: &str,
        partition: i32,
    ) -> Option<Self::PartitionSource> {
        self.partition_sources
            .lock()
            .unwrap()
            .remove(&TopicPartition {
                topic: topic.to_string(),
                partition,
            })
    }
}

fn message(topic: &str, partition: i32, offset: i64, timestamp: i64) -> OwnedMessage {
    OwnedMessage::new(
        None,
        None,
        topic.to_string(),
        Timestamp::CreateTime(timestamp),
        partition,
        offset,
        None,
    )
}
