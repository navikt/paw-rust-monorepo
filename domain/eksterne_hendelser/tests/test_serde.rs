use kafka_broker_testcontainer::kafka_broker::setup_kafka_broker_container;
use schema_registry_testcontainer::schema_registry::setup_schema_registry_container;

#[tokio::test]
async fn test_send_messages() -> anyhow::Result<()> {
    let broker_guard = setup_kafka_broker_container(9092).await?;
    let registry_guard = setup_schema_registry_container(8887).await?;
    Ok(())
}
