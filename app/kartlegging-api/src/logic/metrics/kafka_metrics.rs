use prometheus::{register_gauge_vec, GaugeVec};
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use std::sync::LazyLock;

static KAFKA_OFFSET_GAUGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_kafka_consumer_message_offset",
        "Siste offset for Kafka consumer",
        &["topic", "partition"]
    )
    .expect("Failed to register kafka_consumer_message_offset gauge")
});

pub(crate) fn init() {}

pub fn register_message_metrics(message: &OwnedMessage) {
    KAFKA_OFFSET_GAUGE
        .with_label_values(&[&message.topic(), &message.partition().to_string().as_str()])
        .set(message.offset() as f64);
}
