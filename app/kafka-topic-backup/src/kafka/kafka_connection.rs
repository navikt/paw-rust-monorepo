use anyhow::Result;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka::kafka_config::KafkaConfig;
use paw_rdkafka_hwm::rebalance::{
    hwm_rebalance_handler::HwmRebalanceHandler, topic_partition_update::TopicPartitionUpdate,
};
use rdkafka::consumer::{Consumer, StreamConsumer};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub fn create_kafka_consumer(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
) -> Result<StreamConsumer<HwmRebalanceHandler>> {
    create_kafka_consumer_inner(app_state, pg_pool, kafka_config, topics, None)
}
pub fn create_kafka_consumer_with_sender(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
    sender: UnboundedSender<TopicPartitionUpdate>,
) -> Result<StreamConsumer<HwmRebalanceHandler>> {
    create_kafka_consumer_inner(app_state, pg_pool, kafka_config, topics, Some(sender))
}

fn create_kafka_consumer_inner(
    app_state: Arc<AppState>,
    pg_pool: PgPool,
    kafka_config: KafkaConfig,
    topics: &[&str],
    sender: Option<UnboundedSender<TopicPartitionUpdate>>,
) -> Result<StreamConsumer<HwmRebalanceHandler>> {
    let config = kafka_config.rdkafka_client_config()?;
    let context = HwmRebalanceHandler {
        pg_pool,
        app_state,
        version: *kafka_config.hwm_version,
        sender,
    };
    let consumer: StreamConsumer<HwmRebalanceHandler> = config.create_with_context(context)?;
    consumer.subscribe(topics)?;
    Ok(consumer)
}
