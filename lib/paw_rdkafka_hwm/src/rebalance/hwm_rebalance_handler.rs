use crate::rebalance::get_hwms::get_hwms;
use crate::rebalance::topic_partition_update::{TopicPartition, TopicPartitionUpdate};
use health_and_monitoring::simple_app_state::AppState;
use rdkafka::consumer::ConsumerContext;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::types::RDKafkaRespErr;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

pub struct HwmRebalanceHandler {
    pub pg_pool: PgPool,
    pub app_state: Arc<AppState>,
    pub version: i16,
    pub sender: Option<UnboundedSender<TopicPartitionUpdate>>,
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
                        self.send_disconnected_msg();
                        return;
                    }
                };

                for hwm in &hwms {
                    if let Err(e) =
                        tpl.set_partition_offset(&hwm.topic, hwm.partition(), hwm.neste_offset())
                    {
                        tracing::error!(error = %e, "Failed to set partition offset");
                        self.app_state.set_is_alive(false);
                        self.send_disconnected_msg();
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
                        self.send_disconnected_msg();
                    }
                }

                if let Some(sender) = &self.sender
                    && self.app_state.is_alive()
                {
                    sender.send(TopicPartitionUpdate::Assigned {
                        topic_partition_hwms: hwms
                            .into_iter()
                            .map(|hwm| {
                                let partition = hwm.partition();
                                (
                                    TopicPartition {
                                        topic: hwm.topic,
                                        partition,
                                    },
                                    hwm.offset.unwrap_or(crate::hwm::DEFAULT_HWM_OFFSET),
                                )
                            })
                            .collect(),
                    }).unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to send assigned partitions message");
                        self.app_state.set_is_alive(false);
                    });
                }
            }

            RDKafkaRespErr::RD_KAFKA_RESP_ERR__REVOKE_PARTITIONS => {
                tracing::info!(partitions = ?tpl_as_string(tpl), "Partitions revoked");

                match base_consumer.unassign() {
                    Ok(_) => tracing::info!("Unassigned partitions from consumer"),
                    Err(e) => {
                        tracing::error!(error = %e, "Failed to unassign partitions");
                        self.app_state.set_is_alive(false);
                        self.send_disconnected_msg();
                    }
                }

                if let Some(sender) = &self.sender
                    && self.app_state.is_alive()
                {
                    sender.send(TopicPartitionUpdate::Revoked {
                        topic_partitions: tpl
                            .elements()
                            .iter()
                            .map(|tp| crate::rebalance::topic_partition_update::TopicPartition {
                                topic: tp.topic().to_string(),
                                partition: tp.partition(),
                            })
                            .collect(),
                    }).unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to send revoked partitions message");
                        self.app_state.set_is_alive(false);
                    });
                }
            }

            RDKafkaRespErr::RD_KAFKA_RESP_ERR__ASSIGNMENT_LOST => {
                tracing::error!("Assignment lost - Shutting down app");
                self.app_state.set_is_alive(false);
                self.send_disconnected_msg();
            }

            _ => {
                tracing::error!(error = ?err, "Unexpected rebalance signal");
                self.app_state.set_is_alive(false);
                self.send_disconnected_msg();
            }
        }
    }
}

impl HwmRebalanceHandler {
    pub fn new(pg_pool: PgPool, app_state: Arc<AppState>, version: i16) -> Self {
        Self {
            pg_pool,
            app_state,
            version,
            sender: None,
        }
    }

    pub fn new_with_sender(
        pg_pool: PgPool,
        app_state: Arc<AppState>,
        version: i16,
        sender: UnboundedSender<TopicPartitionUpdate>,
    ) -> Self {
        Self {
            pg_pool,
            app_state,
            version,
            sender: Some(sender),
        }
    }
    fn send_disconnected_msg(&self) {
        if let Some(sender) = self.sender.as_ref() {
            sender.send(TopicPartitionUpdate::InternalReceiverDisconnected).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Failed to send internal receiver disconnected message");
            });
        }
    }
}

fn tpl_as_string(topic_partition_list: &TopicPartitionList) -> Vec<String> {
    topic_partition_list
        .elements()
        .iter()
        .map(|tp| format!("{}:{}@{:?}", tp.topic(), tp.partition(), tp.offset()))
        .collect()
}
