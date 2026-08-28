use prometheus::{
    GaugeVec, HistogramVec, IntCounter, IntCounterVec, register_gauge_vec, register_histogram_vec,
    register_int_counter, register_int_counter_vec,
};
use rdkafka::Message;
use rdkafka::message::OwnedMessage;
use std::sync::LazyLock;

static KAFKA_OFFSET_GAUGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_consumer_message_offset",
        "Siste offset for Kafka consumer",
        &["topic", "partition"]
    )
    .expect("Failed to register kafka_consumer_message_offset gauge")
});

static KAFKA_PARTITIONS_PAUSED_GAUGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_partition_paused",
        "1 dersom partisjonen for øyeblikket er pauset fordi den avventer periode, ellers 0",
        &["topic", "partition"]
    )
    .expect("Failed to register kafka_partition_paused gauge")
});

static KAFKA_PARTITION_PAUSE_EVENTS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "paw_kafka_partition_pause_events_total",
        "Antall ganger en partisjon er pauset fordi den avventer periode",
        &["topic", "partition"]
    )
    .expect("Failed to register kafka_partition_pause_events_total counter")
});

static KAFKA_PARTITION_RESUME_EVENTS: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "paw_kafka_partition_resume_events_total",
        "Antall ganger pausede partisjoner er gjenopptatt (etter periode-gjennombrudd eller sikkerhetsventil)"
    )
    .expect("Failed to register kafka_partition_resume_events_total counter")
});

static KAFKA_PARTITION_STUCK_EVENTS: LazyLock<IntCounter> = LazyLock::new(|| {
    register_int_counter!(
        "paw_kafka_partition_stuck_events_total",
        "Antall ganger en partisjon har vært pauset lenger enn terskelen og appen er markert usunn"
    )
    .expect("Failed to register kafka_partition_stuck_events_total counter")
});

static KAFKA_PARTITION_PAUSE_DURATION: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        "paw_kafka_partition_pause_duration_seconds",
        "Hvor lenge en partisjon sto pauset før den ble gjenopptatt",
        &["topic"]
    )
    .expect("Failed to register kafka_partition_pause_duration_seconds histogram")
});

pub(crate) fn init() {}

pub fn register_message_metrics(message: &OwnedMessage) {
    KAFKA_OFFSET_GAUGE
        .with_label_values(&[&message.topic(), &message.partition().to_string().as_str()])
        .set(message.offset() as f64);
}

pub fn record_partition_paused(topic: &str, partition: i32) {
    let partition = partition.to_string();
    KAFKA_PARTITIONS_PAUSED_GAUGE
        .with_label_values(&[topic, &partition])
        .set(1.0);
    KAFKA_PARTITION_PAUSE_EVENTS
        .with_label_values(&[topic, &partition])
        .inc();
}

pub fn record_partition_resumed(topic: &str, partition: i32, paused_for: std::time::Duration) {
    KAFKA_PARTITIONS_PAUSED_GAUGE
        .with_label_values(&[topic, &partition.to_string()])
        .set(0.0);
    KAFKA_PARTITION_RESUME_EVENTS.inc();
    KAFKA_PARTITION_PAUSE_DURATION
        .with_label_values(&[topic])
        .observe(paused_for.as_secs_f64());
}

pub fn record_partition_stuck() {
    KAFKA_PARTITION_STUCK_EVENTS.inc();
}
