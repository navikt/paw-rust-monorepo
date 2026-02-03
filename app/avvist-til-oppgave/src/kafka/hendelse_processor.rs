use crate::kafka::kafka_message::KafkaMessage;
use crate::kafka::serde::deserialize_value_to_string;
use health_and_monitoring::simple_app_state::AppState;
use paw_rdkafka_hwm::hwm_rebalance_handler::HwmRebalanceHandler;
use rdkafka::consumer::StreamConsumer;
use sqlx::PgPool;
use std::sync::Arc;

pub async fn process_hendelse(
    hendelselogg_consumer: StreamConsumer<HwmRebalanceHandler>,
    pg_pool: PgPool,
    app_state: Arc<AppState>,
) {
    loop {
        let message = hendelselogg_consumer.recv();
        match message.await {
            Ok(borrowed_message) => {
                let kafka_message = KafkaMessage::from_borrowed_message(borrowed_message);
                /*
                let avvist_hendelse = match deserialize_value_to_string(kafka_message) {
                    Ok(hendelse) => {
                        match hendelse.contains("intern.v1.avvist")
                            && hendelse.contains("Er under 18 år")
                        {
                            true => {
                                //TODO: json til hendelsestruct
                            }
                            false => { /*noop*/ }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to deserialize message: {}", e);
                        continue;
                    }

                };
                 */
            }
            Err(e) => {
                log::error!("Error receiving message: {}", e);
                app_state.set_is_alive(false)
            }
        }
    }
}
