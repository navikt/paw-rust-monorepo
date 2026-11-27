use paw_observability::otel::config::OtelTracingConfig;
use paw_observability::otel::format::OtelFormat;
use paw_observability::otel::setup_otel;

pub fn setup_test_tracing() -> anyhow::Result<()> {
    setup_otel(test_config())
}

fn test_config() -> OtelTracingConfig {
    OtelTracingConfig {
        format: OtelFormat::Compact,
        directives: [
            "INFO",
            "tiltak_kafka_adapter::logic=DEBUG",
            "tiltak_kafka_adapter::logic::metrics=INFO",
            "paw_kafka::hwm::rebalance::hwm_rebalance_handler=DEBUG",
        ]
        .map(String::from)
        .to_vec(),
    }
}
