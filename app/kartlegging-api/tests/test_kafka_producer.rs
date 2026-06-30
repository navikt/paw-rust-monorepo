use eksterne_hendelser::bekreftelse::bekreftelse::BEKREFTELSE_TOPIC;
use eksterne_hendelser::periode::PERIODE_TOPIC;
use eksterne_hendelser::serde::AvroSerializer;
use kartlegging_api::config::read_kafka_config;
use nais_schema_registry::config::create_schema_registry_settings;
use rdkafka::producer::FutureProducer;
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let kafka_config = read_kafka_config()?;
    let config = kafka_config.rdkafka_client_config()?;
    let producer: FutureProducer = config.create()?;
    let schema_registry_settings = create_schema_registry_settings()?;
    let serializer = AvroSerializer::new(schema_registry_settings);
    let periode_naming_strategy =
        SubjectNameStrategy::TopicNameStrategy(PERIODE_TOPIC.to_string(), false);
    let bekreftelse_naming_strategy =
        SubjectNameStrategy::TopicNameStrategy(BEKREFTELSE_TOPIC.to_string(), false);
    Ok(())
}
