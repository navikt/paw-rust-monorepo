use crate::hwm::{DEFAULT_HWM, Hwm};
use crate::hwm_functions::{get_hwm, insert_hwm};
use crate::rebalance::rebalance_error::RebalanceError;
use futures::executor::block_on;
use rdkafka::topic_partition_list::TopicPartitionList;
use sqlx::PgPool;

pub(super) fn get_hwms(
    version: i16,
    tpl: &TopicPartitionList,
    pool: &PgPool,
) -> Result<Vec<Hwm>, RebalanceError> {
    block_on(async {
        let mut tx = pool.begin().await.map_err(anyhow::Error::from)?;
        let mut hwms = Vec::new();
        for topic_partition in tpl.elements() {
            let topic = topic_partition.topic();
            let partition = topic_partition.partition();
            let offset = match get_hwm(&mut tx, version, topic, partition).await? {
                Some(offset) => offset,
                None => {
                    insert_hwm(&mut tx, version, topic, partition, DEFAULT_HWM).await?;
                    DEFAULT_HWM
                }
            };
            let hwm = Hwm {
                topic: topic.to_string(),
                partition,
                offset: Some(offset),
            };
            hwms.push(hwm);
        }
        tx.commit().await.map_err(anyhow::Error::from)?;
        Ok(hwms)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use paw_test::setup_test_db::setup_test_db;
    use rdkafka::Offset;

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn ny_partisjon_inserter_default_hwm() {
        let pool = setup_db().await;

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset("topic-a", 0, Offset::Invalid)
            .unwrap();

        let hwms = get_hwms(1, &tpl, &pool).unwrap();

        assert_eq!(hwms.len(), 1);
        assert_eq!(hwms[0].topic, "topic-a");
        assert_eq!(hwms[0].partition, 0);
        assert_eq!(hwms[0].offset, Some(DEFAULT_HWM));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn eksisterende_partisjon_henter_lagret_offset() {
        let pool = setup_db().await;
        let mut tx = pool.begin().await.unwrap();
        insert_hwm(&mut tx, 1, "topic-a", 0, 42).await.unwrap();
        tx.commit().await.unwrap();

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset("topic-a", 0, Offset::Invalid)
            .unwrap();

        let hwms = get_hwms(1, &tpl, &pool).unwrap();

        assert_eq!(hwms.len(), 1);
        assert_eq!(hwms[0].offset, Some(42));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn flere_partisjoner_mix_ny_og_eksisterende() {
        let pool = setup_db().await;
        let mut tx = pool.begin().await.unwrap();
        insert_hwm(&mut tx, 1, "topic-a", 0, 100).await.unwrap();
        tx.commit().await.unwrap();

        let mut tpl = TopicPartitionList::new();
        tpl.add_partition_offset("topic-a", 0, Offset::Invalid)
            .unwrap();
        tpl.add_partition_offset("topic-a", 1, Offset::Invalid)
            .unwrap();
        tpl.add_partition_offset("topic-b", 0, Offset::Invalid)
            .unwrap();

        let hwms = get_hwms(1, &tpl, &pool).unwrap();

        assert_eq!(hwms.len(), 3);
        assert_eq!(hwms[0].offset, Some(100));
        assert_eq!(hwms[1].offset, Some(DEFAULT_HWM));
        assert_eq!(hwms[2].offset, Some(DEFAULT_HWM));
    }

    async fn setup_db() -> PgPool {
        let (pool, _guard) = setup_test_db().await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }
}
