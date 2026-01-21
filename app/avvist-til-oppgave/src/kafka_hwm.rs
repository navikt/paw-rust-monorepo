use std::error;
use health::simple_app_state::AppState;
use log::error as log_error;
use rdkafka::consumer::{BaseConsumer, Consumer, ConsumerContext, Rebalance};
use rdkafka::{ClientContext, Offset};
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;

struct Hwm {
    topic: String,
    partition: i32,
    offset: i64,
}

struct TopicPartition {
    topic: String,
    partition: i32,
}

struct HwmRebalanceHandler {
    pub pg_pool: PgPool,
    pub app_state: Arc<AppState>,
}

pub async fn get_hwm(
    tx: &mut Transaction<'_, Postgres>,
    topic: &str,
    partition: i32,
) -> Result<Option<i64>, Box<dyn error::Error>> {
    let row = sqlx::query_scalar(
        r#"
        SELECT offset
        FROM kafka_hwms
        WHERE topic = $1 AND partition = $2
        "#,
    )
        .bind(topic)
        .bind(partition)
        .fetch_optional(&mut **tx)
        .await?;

    Ok(row)
}

impl ClientContext for HwmRebalanceHandler {}
impl ConsumerContext for HwmRebalanceHandler {
    fn post_rebalance(&self, base_consumer: &BaseConsumer<Self>, rebalance: &Rebalance<'_>) {
        match rebalance {
            Rebalance::Assign(partitions) => {
                let topic_partitions = partitions.elements().iter().map(|tp| TopicPartition {
                    topic: tp.topic().to_string(),
                    partition: tp.partition(),
                }).collect();

                let hwms = futures::executor::block_on(self.get_hwms(topic_partitions)).unwrap();
                hwms.iter().for_each(|hwm| {
                    log::info!("HWM for topic {} partition {} is {}", hwm.topic, hwm.partition, hwm.offset);
                    base_consumer.seek(
                        &hwm.topic,
                        hwm.partition,
                        hwm.seek_to_rd_kafka_offset(),
                        std::time::Duration::from_secs(10),
                    ).unwrap();
                })
            }
            Rebalance::Revoke(partitions) => {
                log::info!("Partitions revoked: {:?}", partitions);
            }
            Rebalance::Error(kafka_error) => {
                log_error!("Rebalance error: {}", kafka_error);
                self.app_state.set_is_alive(false)
            }
        }
    }
}

impl HwmRebalanceHandler {
    async fn get_hwms(
        &self,
        topic_partitions: Vec<TopicPartition>,
    ) -> Result<Vec<Hwm>, Box<dyn std::error::Error>> {
        let mut tx = self.pg_pool.begin().await?;
        let mut hwms = Vec::new();
        for tp in topic_partitions {
            let offset = get_hwm(&mut tx, &tp.topic, tp.partition).await?;
            let hwm = Hwm {
                topic: tp.topic,
                partition: tp.partition,
                offset: offset.unwrap_or(-1),
            };
            hwms.push(hwm)
        }
        Ok(hwms)
    }
}

impl Hwm {
    fn seek_to_rd_kafka_offset(&self) -> Offset {
        match self.offset {
            -1 => Offset::Beginning,
            _ => Offset::Offset(self.offset + 1),
        }
    }
}
