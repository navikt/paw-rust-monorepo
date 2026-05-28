use crate::rebalance::get_hwms::get_hwms;
use health_and_monitoring::simple_app_state::AppState;
use rdkafka::ClientContext;
use rdkafka::consumer::ConsumerContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::types::RDKafkaRespErr;
use sqlx::PgPool;
use std::sync::Arc;

pub struct HwmRebalanceHandler {
    pub pg_pool: PgPool,
    pub app_state: Arc<AppState>,
    pub version: i16,
}

impl ConsumerContext for HwmRebalanceHandler {
    fn rebalance(
        &self,
        base_consumer: &BaseConsumer<Self>,
        err: RDKafkaRespErr,
        tpl: &mut TopicPartitionList,
    ) {
        match err {
            RDKafkaRespErr::RD_KAFKA_RESP_ERR__ASSIGN_PARTITIONS => {
                tracing::info!(partitions = ?tpl_as_string(tpl), "Partitions assigned");

                let hwms = match get_hwms(self.version, tpl, &self.pg_pool) {
                    Ok(hwms) => hwms,
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to get HWMs");
                        self.app_state.set_is_alive(false);
                        return;
                    }
                };

                for hwm in &hwms {
                    if let Err(e) = tpl.set_partition_offset(&hwm.topic, hwm.partition, hwm.neste_offset()) {
                        tracing::error!(error = %e, "Failed to set partition offset");
                        self.app_state.set_is_alive(false);
                        return;
                    }
                }

                match base_consumer.assign(tpl) {
                    Ok(_) => {
                        tracing::info!(partitions = ?tpl_as_string(tpl), "Consumer assigned with HWM offsets");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to assign partitions");
                        self.app_state.set_is_alive(false);
                    }
                }
            }

            RDKafkaRespErr::RD_KAFKA_RESP_ERR__REVOKE_PARTITIONS => {
                tracing::info!(partitions = ?tpl_as_string(tpl), "Partitions revoked");

                match base_consumer.unassign() {
                    Ok(_) => tracing::info!("Unassigned partitions from consumer"),
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to unassign partitions");
                        self.app_state.set_is_alive(false);
                    }
                }
            }

            RDKafkaRespErr::RD_KAFKA_RESP_ERR__ASSIGNMENT_LOST => {
                tracing::error!("Assignment lost - Shutting down app");
                self.app_state.set_is_alive(false);
            }

            _ => {
                tracing::error!(error = ?err, "Unexpected rebalance signal");
                self.app_state.set_is_alive(false);
            }
        }
    }
}

impl ClientContext for HwmRebalanceHandler {}

fn tpl_as_string(topic_partition_list: &TopicPartitionList) -> Vec<String> {
    topic_partition_list
        .elements()
        .iter()
        .map(|tp| format!("{}:{}@{:?}", tp.topic(), tp.partition(), tp.offset()))
        .collect()
}
