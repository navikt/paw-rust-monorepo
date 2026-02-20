use crate::kafka::headers::{extract_headers_as_map, extract_remote_otel_context};
use paw_rdkafka_hwm::hwm_functions::update_hwm;
use prometheus::{register_counter_vec, CounterVec};
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use sqlx::{PgPool, Postgres, Transaction};
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::OnceLock;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub type ProcessorError = Box<dyn Error + Send + Sync>;
pub trait MessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        msg: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = Result<(), ProcessorError>> + Send + 'a>>;
}

pub async fn hwm_process_message(
    hwm_version: i16,
    pg_pool: PgPool,
    msg: &OwnedMessage,
    processor: &(dyn MessageProcessor + Send + Sync),
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let span_name = format!("{} process", msg.topic());
    let span = tracing::info_span!(
        "kafka_message_process",
        otel.name = span_name.as_str(),
        messaging.system = "kafka",
        messaging.destination.name = msg.topic(),
        messaging.destination.partition.id = msg.partition(),
        messaging.kafka.message.offset = msg.offset()
    );
    let mut tx = pg_pool.begin().await?;
    let topic = &msg.topic();
    let headers = extract_headers_as_map(&msg);
    let remote_trace_context = extract_remote_otel_context(&headers);
    if let Some(remote_ctx) = remote_trace_context {
        match span.set_parent(remote_ctx) {
            Ok(_) => tracing::debug!("Successfully set parent context for span"),
            Err(e) => tracing::error!("Failed to set parent context for span: {}", e),
        }
    }
    let hwm_ok = update_hwm(&mut tx, hwm_version, topic, msg.partition(), msg.offset()).await?;

    if hwm_ok {
        let res = processor.process_message(&mut tx, msg).await;
        increment_kafka_messages_processed(true, &topic.to_string(), msg.partition(), res.is_err());
        match res {
            Ok(_) => tracing::debug!(
                "Message processed successfully: topic={}, partition={}, offset={}",
                topic,
                msg.partition(),
                msg.offset()
            ),
            Err(e) => {
                tracing::error!(
                    "Error processing message: topic={}, partition={}, offset={}, error={}",
                    topic,
                    msg.partition(),
                    msg.offset(),
                    e
                );
                tx.rollback().await?;
                return Err(e);
            }
        }
        tx.commit().await?;
    } else {
        increment_kafka_messages_processed(false, &topic.to_string(), msg.partition(), false);
        tracing::info!(
            "Below HWM, topic={}, partition={}, offset={}",
            topic,
            msg.partition(),
            msg.offset()
        );
    }
    Ok(())
}

static KAFKA_MESSAGES_PROCESSED: OnceLock<CounterVec> = OnceLock::new();

pub fn increment_kafka_messages_processed(
    above_hwm: bool,
    topic: &String,
    partition: i32,
    error: bool,
) {
    let counter_vec = KAFKA_MESSAGES_PROCESSED.get_or_init(|| {
        register_counter_vec!(
            "kafka_messages_processed_total",
            "Total number of Kafka messages processed",
            &["above_hwm", "topic", "partition", "error"]
        )
        .expect("Failed to register kafka_messages_processed_total counter")
    });
    counter_vec
        .with_label_values(&[
            &above_hwm.to_string(),
            topic,
            &partition.to_string(),
            &error.to_string(),
        ])
        .inc();
}
