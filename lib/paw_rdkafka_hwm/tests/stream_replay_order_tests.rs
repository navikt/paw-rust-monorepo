use std::collections::{HashMap, VecDeque};
use std::future::{Future, pending};
use std::sync::{Arc, Mutex as StdMutex};
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
use tokio::time::{Instant, sleep_until};

const TOPIC_COUNT: usize = 8;
const MESSAGES_PER_TOPIC: usize = 250;

#[tokio::test]
async fn replay_hopper_ikke_bakover_i_tid() {
    let mut assigned = Vec::with_capacity(TOPIC_COUNT);
    let mut high_watermarks = Vec::with_capacity(TOPIC_COUNT);
    let mut sources = Vec::with_capacity(TOPIC_COUNT);

    for topic_index in 0..TOPIC_COUNT {
        let topic_partition = TopicPartition {
            topic: format!("topic-{topic_index}"),
            partition: 0,
        };
        let topic = topic_partition.topic.clone();
        let messages = (0..MESSAGES_PER_TOPIC).map(move |offset| {
            message(
                &topic,
                offset as i64,
                (offset * TOPIC_COUNT + topic_index) as i64,
            )
        });
        assigned.push((topic_partition.clone(), -1));
        high_watermarks.push(offsets(topic_partition.clone(), MESSAGES_PER_TOPIC as i64));
        sources.push((topic_partition, FakePartitionSource::new(messages)));
    }

    let consumer = FakeConsumer::new(sources);
    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: assigned,
        })
        .unwrap();
    sender
        .send(TopicPartitionUpdate::HiOffsetUpdate {
            topic_partition_offsets: high_watermarks,
        })
        .unwrap();

    let stream = PawKafkaConsumerStream::new(
        receiver,
        consumer,
        Duration::from_millis(10),
        7,
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        1,
        1,
    );

    let expected_count = TOPIC_COUNT * MESSAGES_PER_TOPIC;
    let mut stream = stream;
    let mut received = Vec::with_capacity(expected_count);
    let mut receive_calls = 0;
    while received.len() < expected_count {
        receive_calls += 1;
        assert!(
            receive_calls <= expected_count * 2,
            "stream stopped making progress after {} messages",
            received.len()
        );
        let (next_stream, message) = stream.receive().await.unwrap();
        stream = next_stream;
        if let Some(message) = message {
            received.push((
                message.topic().to_string(),
                message.offset(),
                message.timestamp().to_millis().unwrap(),
            ));
        }
    }

    for messages in received.windows(2) {
        let previous = &messages[0];
        let current = &messages[1];
        assert!(
            previous.2 <= current.2,
            "timestamp moved backwards from {previous:?} to {current:?}"
        );
    }
}

#[tokio::test]
async fn next_offset_forbi_control_record_blokkerer_ikke_andre_koer() {
    let transactional_topic = TopicPartition {
        topic: "transactional-topic".to_string(),
        partition: 0,
    };
    let active_topic = TopicPartition {
        topic: "active-topic".to_string(),
        partition: 0,
    };
    let consumer = FakeConsumer::new([
        (
            transactional_topic.clone(),
            FakePartitionSource::new_pending_when_empty([message(
                &transactional_topic.topic,
                0,
                100,
            )]),
        ),
        (
            active_topic.clone(),
            FakePartitionSource::new(
                (0..10).map(|offset| message(&active_topic.topic, offset, 200 + offset)),
            ),
        ),
    ]);
    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: vec![
                (transactional_topic.clone(), -1),
                (active_topic.clone(), -1),
            ],
        })
        .unwrap();
    sender
        .send(TopicPartitionUpdate::HiOffsetUpdate {
            topic_partition_offsets: vec![
                offsets(transactional_topic.clone(), 2),
                offsets(active_topic, 10),
            ],
        })
        .unwrap();

    let stream = PawKafkaConsumerStream::new(
        receiver,
        consumer,
        Duration::from_millis(1),
        8,
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        1,
        1,
    );

    let (stream, first) = stream.receive().await.unwrap();
    let first = first.unwrap();
    assert_eq!(first.topic(), transactional_topic.topic);
    assert_eq!(first.offset(), 0);

    let (_, unblocked) = stream.receive().await.unwrap();
    assert_eq!(unblocked.unwrap().topic(), "active-topic");
}

#[tokio::test]
#[ignore = "slow randomized replay test"]
async fn fuzz_replay_hopper_ikke_bakover_i_tid() {
    for seed in 0..250 {
        run_fuzz_case(seed).await;
    }
}

#[tokio::test(start_paused = true)]
#[ignore = "slow randomized replay test"]
async fn fuzz_replay_med_reelle_timestamps_holder_seg_under_to_sekunder() {
    let mut largest_jump = None;
    for seed in 0..250 {
        largest_jump = largest_jump.max(run_high_watermark_fuzz_case(seed).await);
    }

    let (jump_ms, seed, previous, current) = largest_jump.unwrap_or_default();
    println!(
        "largest backwards jump: {jump_ms} ms at seed {seed}, from {previous:?} to {current:?}"
    );
    assert!(
        jump_ms <= 2_000,
        "timestamp moved {jump_ms} ms backwards at seed {seed}, from {previous:?} to {current:?}"
    );
}

async fn run_fuzz_case(seed: u64) {
    let mut random = Random::new(seed + 1);
    let topic_count = random.range(2, 13);
    let internal_buffer_size = random.range(2, 33);
    let mut assigned = Vec::with_capacity(topic_count);
    let mut high_watermarks = Vec::with_capacity(topic_count);
    let mut sources = Vec::with_capacity(topic_count);
    let mut expected_count = 0;

    for topic_index in 0..topic_count {
        let message_count = random.range(10, 81);
        expected_count += message_count;
        let topic_partition = TopicPartition {
            topic: format!("topic-{topic_index}"),
            partition: 0,
        };
        let mut timestamp = random.range(0, 10_000) as i64;
        let mut messages = Vec::with_capacity(message_count);
        for offset in 0..message_count {
            timestamp += random.range(1, 100) as i64;
            let delay = Duration::from_micros(random.range(0, 2_001) as u64);
            messages.push((
                message(&topic_partition.topic, offset as i64, timestamp),
                delay,
            ));
        }
        assigned.push((topic_partition.clone(), -1));
        high_watermarks.push(offsets(topic_partition.clone(), message_count as i64));
        sources.push((
            topic_partition,
            FakePartitionSource::new_scheduled(messages),
        ));
    }

    let consumer = FakeConsumer::new(sources);
    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: assigned,
        })
        .unwrap();
    sender
        .send(TopicPartitionUpdate::HiOffsetUpdate {
            topic_partition_offsets: high_watermarks,
        })
        .unwrap();

    let mut stream = PawKafkaConsumerStream::new(
        receiver,
        consumer,
        Duration::from_millis(1),
        internal_buffer_size,
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        1,
        random.range(1, 11),
    );
    let mut received = Vec::with_capacity(expected_count);
    let mut receive_calls = 0;
    while received.len() < expected_count {
        receive_calls += 1;
        assert!(
            receive_calls < expected_count * 100,
            "seed {seed} stopped making progress after {} messages",
            received.len()
        );
        let (next_stream, message) = stream.receive().await.unwrap();
        stream = next_stream;
        if let Some(message) = message {
            received.push((
                message.topic().to_string(),
                message.offset(),
                message.timestamp().to_millis().unwrap(),
            ));
        }
    }

    for messages in received.windows(2) {
        let previous = &messages[0];
        let current = &messages[1];
        assert!(
            previous.2 <= current.2,
            "seed {seed}, buffer size {internal_buffer_size}: timestamp moved backwards from {previous:?} to {current:?}"
        );
    }
}

async fn run_high_watermark_fuzz_case(
    seed: u64,
) -> Option<(i64, u64, (String, i64, i64), (String, i64, i64))> {
    let mut random = Random::new(seed + 10_000);
    let topic_count = random.range(2, 13);
    let simulation_seconds = random.range(5, 11);
    let mut assigned = Vec::with_capacity(topic_count);
    let mut sources = Vec::with_capacity(topic_count);
    let mut production_times = Vec::with_capacity(topic_count);
    let mut expected_count = 0;

    for topic_index in 0..topic_count {
        let topic_partition = TopicPartition {
            topic: format!("topic-{topic_index}"),
            partition: 0,
        };
        let mut produced_at_ms = random.range(1, 1_000) as i64;
        let mut previous_at_ms = 0;
        let mut messages = Vec::new();
        let mut topic_production_times = Vec::new();
        while produced_at_ms <= simulation_seconds as i64 * 1_000 {
            let offset = messages.len() as i64;
            messages.push((
                message(&topic_partition.topic, offset, produced_at_ms),
                Duration::from_millis((produced_at_ms - previous_at_ms) as u64),
            ));
            topic_production_times.push(produced_at_ms);
            previous_at_ms = produced_at_ms;
            produced_at_ms += random.range(10, 501) as i64;
        }
        expected_count += messages.len();
        assigned.push((topic_partition.clone(), -1));
        sources.push((
            topic_partition.clone(),
            FakePartitionSource::new_scheduled(messages),
        ));
        production_times.push((topic_partition, topic_production_times));
    }

    let (sender, receiver) = mpsc::unbounded_channel();
    sender
        .send(TopicPartitionUpdate::Assigned {
            topic_partition_hwms: assigned,
        })
        .unwrap();
    sender
        .send(TopicPartitionUpdate::HiOffsetUpdate {
            topic_partition_offsets: production_times
                .iter()
                .map(|(topic_partition, _)| offsets(topic_partition.clone(), 0))
                .collect(),
        })
        .unwrap();

    let statistics_sender = sender.clone();
    let mut previous_high_watermarks = vec![0; topic_count];
    let mut snapshots = Vec::with_capacity(simulation_seconds + 1);
    for second in 1..=simulation_seconds {
        let snapshot_at_ms = second as i64 * 1_000;
        let mut snapshot = Vec::with_capacity(topic_count);
        for (topic_index, (topic_partition, topic_times)) in production_times.iter().enumerate() {
            let collection_skew_ms = random.range(0, 1_001) as i64;
            let sampled_at_ms = snapshot_at_ms - collection_skew_ms;
            let high_watermark = topic_times.partition_point(|time| *time <= sampled_at_ms) as i64;
            previous_high_watermarks[topic_index] =
                previous_high_watermarks[topic_index].max(high_watermark);
            snapshot.push(offsets(
                topic_partition.clone(),
                previous_high_watermarks[topic_index],
            ));
        }
        snapshots.push(snapshot);
    }
    snapshots.push(
        production_times
            .iter()
            .map(|(topic_partition, topic_times)| {
                offsets(topic_partition.clone(), topic_times.len() as i64)
            })
            .collect(),
    );
    tokio::spawn(async move {
        for snapshot in snapshots {
            tokio::time::sleep(Duration::from_secs(1)).await;
            statistics_sender
                .send(TopicPartitionUpdate::HiOffsetUpdate {
                    topic_partition_offsets: snapshot,
                })
                .unwrap();
        }
    });

    let mut stream = PawKafkaConsumerStream::new(
        receiver,
        FakeConsumer::new(sources),
        Duration::from_millis(10),
        random.range(2, 10),
        PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap(),
        1,
        1,
    );
    let mut received = Vec::with_capacity(expected_count);
    while received.len() < expected_count {
        let (next_stream, output) = stream.receive().await.unwrap();
        stream = next_stream;
        if let Some(message) = output {
            received.push((
                message.topic().to_string(),
                message.offset(),
                message.timestamp().to_millis().unwrap(),
            ));
        }
    }

    received
        .windows(2)
        .filter_map(|messages| {
            let previous = &messages[0];
            let current = &messages[1];
            (previous.2 > current.2).then(|| {
                (
                    previous.2 - current.2,
                    seed,
                    previous.clone(),
                    current.clone(),
                )
            })
        })
        .max_by_key(|jump| jump.0)
}

#[derive(Clone)]
struct FakePartitionSource {
    messages: Arc<StdMutex<VecDeque<ScheduledMessage>>>,
}

impl FakePartitionSource {
    fn new(messages: impl IntoIterator<Item = OwnedMessage>) -> Self {
        Self::new_scheduled(
            messages
                .into_iter()
                .map(|message| (message, Duration::ZERO)),
        )
    }

    fn new_scheduled(messages: impl IntoIterator<Item = (OwnedMessage, Duration)>) -> Self {
        let mut available_at = Instant::now();
        Self {
            messages: Arc::new(StdMutex::new(
                messages
                    .into_iter()
                    .map(|(message, delay)| {
                        available_at += delay;
                        ScheduledMessage {
                            message,
                            available_at,
                        }
                    })
                    .collect(),
            )),
        }
    }

    fn new_pending_when_empty(messages: impl IntoIterator<Item = OwnedMessage>) -> Self {
        Self::new(messages)
    }
}

impl PartitionMessageSource for FakePartitionSource {
    async fn recv(&self) -> Result<OwnedMessage, StreamError> {
        let available_at = self
            .messages
            .lock()
            .unwrap()
            .front()
            .map(|message| message.available_at);
        let Some(available_at) = available_at else {
            return pending().await;
        };
        if available_at > Instant::now() {
            sleep_until(available_at).await;
        }
        self.messages
            .lock()
            .unwrap()
            .pop_front()
            .map(|message| message.message)
            .ok_or_else(|| StreamError::FailedToReadRecord("fake source is empty".to_string()))
    }
}

struct ScheduledMessage {
    message: OwnedMessage,
    available_at: Instant,
}

struct FakeConsumer {
    partition_sources: StdMutex<HashMap<TopicPartition, FakePartitionSource>>,
}

impl FakeConsumer {
    fn new(
        partition_sources: impl IntoIterator<Item = (TopicPartition, FakePartitionSource)>,
    ) -> Self {
        Self {
            partition_sources: StdMutex::new(partition_sources.into_iter().collect()),
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

struct Random(u64);

impl Random {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn range(&mut self, start: usize, end: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        start + self.0 as usize % (end - start)
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

fn offsets(topic_partition: TopicPartition, hi_offset: i64) -> (TopicPartition, KafkaOffsets) {
    (
        topic_partition,
        KafkaOffsets {
            hi_offset,
            next_offset: hi_offset,
        },
    )
}
