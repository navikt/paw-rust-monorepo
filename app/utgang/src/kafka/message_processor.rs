use rdkafka::Message;
use rdkafka::message::{OwnedMessage};
use sqlx::PgPool;
use std::error::Error;
use sqlx::types::chrono::{DateTime, Utc};
use paw_rdkafka_hwm::hwm_functions::update_hwm;

pub async fn prosesser_melding(pg_pool: PgPool, msg: OwnedMessage) -> Result<(), Box<dyn Error>> {
    let mut tx = pg_pool.begin().await?;
    let topic = &msg.topic();

    let hwm_ok = update_hwm(&mut tx, 1, topic, msg.partition(), msg.offset()).await?;

    if hwm_ok {
        todo!("insert data");
        tx.commit().await?;
        tracing::trace!(
            "Message processed: topic={}, partition={}, offset={}",
            topic, msg.partition(), msg.offset()
        );
    } else {
        tracing::info!(
            "Below HWM, skipping insert: topic={}, partition={}, offset={}",
            topic, msg.partition(), msg.offset()
        );
        tx.rollback().await?;
    }
    Ok(())
}
