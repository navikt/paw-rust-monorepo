use std::collections::{HashMap, VecDeque};
use std::future::{Future, pending};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use paw_rdkafka_hwm::rebalance::topic_partition_update::{
    KafkaOffsets, TopicPartition, TopicPartitionUpdate,
};
use paw_rdkafka_hwm::stream::paw_kafka_stream::{PawKafkaStream, StreamError};
use paw_rdkafka_hwm::stream::queue_handler::PartitionMessageSource;
use paw_rdkafka_hwm::stream::stream_config::PawKafkaStreamConfig;
use paw_rdkafka_hwm::stream::stream_wrapper::{ConsumerMessageSource, PawKafkaConsumerStream};
use paw_rdkafka_hwm::stream::topic_priority::TopicPriorityList;
use rdkafka::Message;
use rdkafka::message::{OwnedMessage, Timestamp};
use sqlx::postgres::PgPoolOptions;
use tokio::sync::mpsc;

#[tokio::test]
async fn likt_timestamp_leveres_i_prioritert_rekkefolge() {
    let topic_uten_prioritet = topic_partition("topic-uten-prioritet");
    let topic_middels = topic_partition("topic-middels");
    let topic_hoyest = topic_partition("topic-hoyest");

    let consumer = FakeConsumer::new([
        (
            topic_uten_prioritet.clone(),
            FakePartitionSource::new([message("topic-uten-prioritet", 0, 100)]),
        ),
        (
            topic_middels.clone(),
            FakePartitionSource::new([message("topic-middels", 0, 100)]),
        ),
        (
            topic_hoyest.clone(),
            FakePartitionSource::new([message("topic-hoyest", 0, 100)]),
        ),
    ]);

    let (stream, _sender) = stream(
        consumer,
        vec![
            topic_uten_prioritet.clone(),
            topic_middels.clone(),
            topic_hoyest.clone(),
        ],
        TopicPriorityList::new(vec![
            ("topic-hoyest".to_string(), 1),
            ("topic-middels".to_string(), 50),
        ]),
    );

    let (stream, forste) = stream.receive().await.unwrap();
    let (stream, andre) = stream.receive().await.unwrap();
    let (_, tredje) = stream.receive().await.unwrap();

    assert_eq!(forste.unwrap().topic(), "topic-hoyest");
    assert_eq!(andre.unwrap().topic(), "topic-middels");
    assert_eq!(tredje.unwrap().topic(), "topic-uten-prioritet");
}

#[tokio::test]
async fn timestamp_veier_tyngre_enn_prioritet() {
    let topic_hoyest = topic_partition("topic-hoyest");
    let topic_lavest = topic_partition("topic-lavest");

    let consumer = FakeConsumer::new([
        (
            topic_hoyest.clone(),
            FakePartitionSource::new([message("topic-hoyest", 0, 300)]),
        ),
        (
            topic_lavest.clone(),
            FakePartitionSource::new([message("topic-lavest", 0, 100)]),
        ),
    ]);

    let (stream, _sender) = stream(
        consumer,
        vec![topic_hoyest.clone(), topic_lavest.clone()],
        TopicPriorityList::new(vec![
            ("topic-hoyest".to_string(), 1),
            ("topic-lavest".to_string(), 900),
        ]),
    );

    let (stream, forste) = stream.receive().await.unwrap();
    let (_, andre) = stream.receive().await.unwrap();

    let forste = forste.unwrap();
    assert_eq!(forste.topic(), "topic-lavest");
    assert_eq!(forste.timestamp(), Timestamp::CreateTime(100));
    assert_eq!(andre.unwrap().topic(), "topic-hoyest");
}

#[tokio::test]
async fn prioritet_brukes_paa_hvert_likt_timestamp() {
    let topic_prioritert = topic_partition("topic-prioritert");
    let topic_vanlig = topic_partition("topic-vanlig");

    let consumer = FakeConsumer::new([
        (
            topic_vanlig.clone(),
            FakePartitionSource::new([
                message("topic-vanlig", 0, 100),
                message("topic-vanlig", 1, 200),
            ]),
        ),
        (
            topic_prioritert.clone(),
            FakePartitionSource::new([
                message("topic-prioritert", 0, 100),
                message("topic-prioritert", 1, 200),
            ]),
        ),
    ]);

    let (mut stream, _sender) = stream(
        consumer,
        vec![topic_vanlig.clone(), topic_prioritert.clone()],
        TopicPriorityList::new(vec![("topic-prioritert".to_string(), 10)]),
    );

    let mut mottatt = Vec::with_capacity(4);
    while mottatt.len() < 4 {
        let (neste, melding) = stream.receive().await.unwrap();
        stream = neste;
        if let Some(melding) = melding {
            mottatt.push((
                melding.topic().to_string(),
                melding.timestamp().to_millis().unwrap(),
            ));
        }
    }

    assert_eq!(
        mottatt,
        vec![
            ("topic-prioritert".to_string(), 100),
            ("topic-vanlig".to_string(), 100),
            ("topic-prioritert".to_string(), 200),
            ("topic-vanlig".to_string(), 200),
        ]
    );
}

fn stream(
    consumer: FakeConsumer,
    tildelte: Vec<TopicPartition>,
    prioriteter: TopicPriorityList,
) -> (
    PawKafkaConsumerStream<FakeConsumer>,
    mpsc::UnboundedSender<TopicPartitionUpdate>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: tildelte
                .iter()
                .map(|topic_partition| (topic_partition.clone(), -1))
                .collect(),
        })
        .unwrap();
    sender
        .send(TopicPartitionUpdate::HiOffsetUpdate {
            topic_partition_offsets: tildelte.into_iter().map(|tp| offsets(tp, 2)).collect(),
        })
        .unwrap();

    let stream = PawKafkaConsumerStream::new(
        receiver,
        consumer,
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        1,
        PawKafkaStreamConfig {
            max_idle: Duration::from_millis(10),
            internal_buffer_size: 10,
            hwm_version: 1,
            topic_priorities: prioriteter,
            grace: Duration::from_millis(1500),
        },
    );
    (stream, sender)
}

fn topic_partition(topic: &str) -> TopicPartition {
    TopicPartition {
        topic: topic.to_string(),
        partition: 0,
    }
}

fn offsets(topic_partition: TopicPartition, hi_offset: i64) -> (TopicPartition, KafkaOffsets) {
    (
        topic_partition,
        KafkaOffsets {
            hi_offset,
            next_offset: hi_offset,
            fetch_queue_count: 0,
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

fn message(topic: &str, offset: i64, timestamp: i64) -> OwnedMessage {
    OwnedMessage::new(
        None,
        None,
        topic.to_string(),
        Timestamp::CreateTime(timestamp),
        0,
        offset,
        None,
    )
}
