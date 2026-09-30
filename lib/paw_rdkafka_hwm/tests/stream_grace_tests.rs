use std::collections::{HashMap, VecDeque};
use std::future::{Future, pending};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

const GRACE: Duration = Duration::from_millis(200);
/// Upper bound for every test, so a stream that never releases a message
/// fails the test instead of hanging it.
const TEST_TIMEOUT: Duration = Duration::from_secs(5);

/// A fresh message is held back while a sibling queue for the same partition
/// is empty, and released once it is `GRACE` old.
#[tokio::test]
async fn fersk_melding_holdes_igjen_til_grace_er_ute_naar_soesterkoe_er_tom() {
    let (stream, _sender) = stream_med_tom_soesterkoe(vec![message("topic-a", 0, now_ms())]);
    let start = Instant::now();

    let (stream, forste) = stream.receive().await.unwrap();
    assert!(
        forste.is_none(),
        "fersk melding ble sluppet før grace var ute"
    );

    let (melding, _) = tokio::time::timeout(TEST_TIMEOUT, motta_neste(stream))
        .await
        .expect("meldingen ble aldri sluppet");
    let ventet = start.elapsed();

    assert_eq!(melding.topic(), "topic-a");
    assert!(
        ventet >= GRACE - Duration::from_millis(20),
        "meldingen ble sluppet etter {ventet:?}, før grace ({GRACE:?}) var ute"
    );
}

/// A message older than `GRACE` is released at once, even when a sibling
/// queue is empty. Replay depends on this.
#[tokio::test]
async fn gammel_melding_slippes_med_en_gang_selv_om_soesterkoe_er_tom() {
    let ti_sekunder_siden = now_ms() - 10_000;
    let (stream, _sender) =
        stream_med_tom_soesterkoe(vec![message("topic-a", 0, ti_sekunder_siden)]);

    let (_, forste) = tokio::time::timeout(TEST_TIMEOUT, stream.receive())
        .await
        .expect("receive hang")
        .unwrap();

    let forste = forste.expect("gammel melding ble holdt igjen");
    assert_eq!(forste.timestamp(), Timestamp::CreateTime(ti_sekunder_siden));
}

/// Grace only applies when a sibling queue is empty. When every queue for the
/// partition has a head message, the oldest one is released at once.
#[tokio::test]
async fn fersk_melding_slippes_med_en_gang_naar_alle_koer_har_melding() {
    let naa = now_ms();
    let topic_a = topic_partition("topic-a");
    let topic_b = topic_partition("topic-b");
    let consumer = FakeConsumer::new([
        (
            topic_a.clone(),
            FakePartitionSource::new([message("topic-a", 0, naa)]),
        ),
        (
            topic_b.clone(),
            FakePartitionSource::new([message("topic-b", 0, naa - 1)]),
        ),
    ]);
    let (stream, _sender) = stream(consumer, vec![(topic_a, 1), (topic_b, 1)], 1);

    let (_, forste) = tokio::time::timeout(TEST_TIMEOUT, stream.receive())
        .await
        .expect("receive hang")
        .unwrap();

    let forste = forste.expect("melding ble holdt igjen selv om ingen søsterkø var tom");
    assert_eq!(forste.topic(), "topic-b");
}

/// Stream with `topic-a` holding `meldinger` and `topic-b` on the same
/// partition empty and caught up. `topic-b` never receives a message, so it
/// needs a second statistics snapshot before `is_lagging()` trusts it.
fn stream_med_tom_soesterkoe(
    meldinger: Vec<OwnedMessage>,
) -> (
    PawKafkaConsumerStream<FakeConsumer>,
    mpsc::UnboundedSender<TopicPartitionUpdate>,
) {
    let topic_a = topic_partition("topic-a");
    let topic_b = topic_partition("topic-b");
    let hi_offset_a = meldinger.len() as i64;
    let consumer = FakeConsumer::new([
        (topic_a.clone(), FakePartitionSource::new(meldinger)),
        (topic_b.clone(), FakePartitionSource::new([])),
    ]);
    stream(consumer, vec![(topic_a, hi_offset_a), (topic_b, 0)], 2)
}

/// Builds a stream with `tildelte` assigned, each with the given `hi_offset`,
/// and sends `snapshots` identical statistics snapshots.
fn stream(
    consumer: FakeConsumer,
    tildelte: Vec<(TopicPartition, i64)>,
    snapshots: usize,
) -> (
    PawKafkaConsumerStream<FakeConsumer>,
    mpsc::UnboundedSender<TopicPartitionUpdate>,
) {
    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: tildelte.iter().map(|(tp, _)| (tp.clone(), -1)).collect(),
        })
        .unwrap();
    for _ in 0..snapshots {
        sender
            .send(TopicPartitionUpdate::HiOffsetUpdate {
                topic_partition_offsets: tildelte
                    .iter()
                    .map(|(tp, hi)| offsets(tp.clone(), *hi))
                    .collect(),
            })
            .unwrap();
    }

    let stream = PawKafkaConsumerStream::new(
        receiver,
        consumer,
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        PawKafkaStreamConfig {
            max_idle: Duration::from_millis(50),
            internal_buffer_size: 10,
            hwm_version: 1,
            main_consumer_none_treshold: 1,
            topic_priorities: TopicPriorityList::empty(),
            grace: GRACE,
        },
    );
    (stream, sender)
}

async fn motta_neste(
    mut stream: PawKafkaConsumerStream<FakeConsumer>,
) -> (OwnedMessage, PawKafkaConsumerStream<FakeConsumer>) {
    loop {
        let (neste, melding) = stream.receive().await.unwrap();
        stream = neste;
        if let Some(melding) = melding {
            return (melding, stream);
        }
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
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
