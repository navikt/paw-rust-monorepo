use prometheus::{CounterVec, register_counter_vec};
use std::sync::OnceLock;

static KAFKA_MESSAGES_PROCESSED: OnceLock<CounterVec> = OnceLock::new();

pub fn init_metrics() {
    KAFKA_MESSAGES_PROCESSED.get_or_init(|| {
        register_counter_vec!(
            "kafka_messages_processed_total",
            "Total number of Kafka messages processed",
            &["above_hwm", "topic", "partition"]
        )
        .expect("Failed to register kafka_messages_processed_total counter")
    });
}

pub fn increment_kafka_messages_processed(above_hwm: bool, topic: &str, partition: i32) {
    if let Some(counter_vec) = KAFKA_MESSAGES_PROCESSED.get() {
        let above_hwm = above_hwm.to_string();
        let partition = partition.to_string();
        counter_vec
            .with_label_values(&[above_hwm.as_str(), topic, partition.as_str()])
            .inc();
    }
}
