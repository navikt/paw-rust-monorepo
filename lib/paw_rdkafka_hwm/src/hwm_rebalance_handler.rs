
use futures::executor::block_on;
use health_and_monitoring::simple_app_state::AppState;
use log::error as log_error;
use rdkafka::ClientContext;
use rdkafka::consumer::{BaseConsumer, Consumer, ConsumerContext, Rebalance};
use sqlx::{PgPool, Postgres, Transaction};
use std::error;
use std::sync::Arc;
use crate::hwm::{Hwm, TopicPartition, DEFAULT_HWM};
use crate::hwm_functions::{get_hwm, insert_hwm};

pub struct HwmRebalanceHandler {
    pub pg_pool: PgPool,
    pub app_state: Arc<AppState>,
    pub version: i16,
}

impl HwmRebalanceHandler {
    async fn get_hwms(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        topic_partitions: Vec<TopicPartition>,
    ) -> Result<Vec<Hwm>, Box<dyn error::Error>> {
        let mut hwms = Vec::new();
        for tp in topic_partitions {
            let offset = get_hwm(&mut *tx, self.version, &tp.topic, tp.partition).await?;
            let hwm = Hwm {
                topic: tp.topic,
                partition: tp.partition,
                offset: offset.unwrap_or(DEFAULT_HWM),
            };
            hwms.push(hwm)
        }
        Ok(hwms)
    }
}

impl ClientContext for HwmRebalanceHandler {}

impl ConsumerContext for HwmRebalanceHandler {

    fn post_rebalance(&self, base_consumer: &BaseConsumer<Self>, rebalance: &Rebalance<'_>) {
        match rebalance {
            Rebalance::Assign(partitions) => {
                let topic_partitions = partitions
                    .elements()
                    .iter()
                    .map(|tp| TopicPartition {
                        topic: tp.topic().to_string(),
                        partition: tp.partition(),
                    })
                    .collect();
                let mut tx = block_on(self.pg_pool.begin()).unwrap();
                let hwms = block_on(self.get_hwms(&mut tx, topic_partitions)).unwrap();
                hwms.iter().for_each(|hwm| {
                    if hwm.offset == DEFAULT_HWM {
                        log::info!(
                            "Inserting initial HWM for topic {} partition {} as {}",
                            hwm.topic,
                            hwm.partition,
                            hwm.offset
                        );
                        block_on(insert_hwm(
                            &mut tx,
                            self.version,
                            &hwm.topic,
                            hwm.partition,
                            hwm.offset,
                        ))
                            .unwrap()
                    } else {
                        log::info!(
                            "HWM for topic {} partition {} is {}",
                            hwm.topic,
                            hwm.partition,
                            hwm.offset
                        );
                    }
                    base_consumer
                        .seek(
                            &hwm.topic,
                            hwm.partition,
                            hwm.seek_to_rd_kafka_offset(),
                            std::time::Duration::from_secs(10),
                        )
                        .unwrap();
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
