pub(crate) mod kafka_metrics;
pub(crate) mod kartlegging_metrics;
pub mod task;

pub fn setup_metrics() {
    kafka_metrics::init();
    kartlegging_metrics::init();
}
