use health::simple_app_state::AppState;
use rdkafka::Offset;
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

pub async fn get_hwm(tx: &mut Transaction<'_, Postgres>, topic: &str, partition: i32) -> Result<Option<i64>, Box<dyn std::error::Error>> {
    let row = sqlx::query_scalar(
        r#"
        SELECT offset
        FROM kafka_hwms
        WHERE topic = $1 AND partition = $2
        "#,
    ).bind(topic).bind(partition).fetch_optional(&mut **tx).await?;

    Ok(row)
}

impl HwmRebalanceHandler {
    async fn get_hwms(&self, topic_partitions: Vec<TopicPartition>) -> Result<Vec<Hwm>, Box<dyn std::error::Error>> {
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
