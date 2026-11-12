use chrono::{DateTime, Utc};
use nais_schema_registry::config::create_schema_registry_settings;
use paw_avro_serde::serde::AvroSerializer;
use rdkafka::message::ToBytes;
use rdkafka::producer::future_producer::Delivery;
use rdkafka::producer::{FutureProducer, FutureRecord};
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use serde::Serialize;
use std::str::FromStr;
use std::time::Duration;
use test_data_generator::eksterne_hendelser::{
    create_dummy_avslutt_periode, create_dummy_start_periode, datetime_rfc3339,
};
use tiltak_kafka_adapter::config::read_kafka_config;
use uuid::Uuid;

#[ignore]
#[tokio::test]
async fn test_send_messages() -> anyhow::Result<()> {
    let data = TestData::gen_test_data();
    let producer = TestKafkaProducer::new()?;

    // producer.send_start_perioder(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
     producer.send_avslutt_perioder(&data).await?;
    //

    Ok(())
}

#[allow(unused)]
struct TestData {
    key: i64,
    arbeidssoeker_id: i64,
    aktor_id: &'static str,
    identitetsnummer: &'static str,
    fornavn: &'static str,
    etternavn: &'static str,
    periode_id: Uuid,
    periode_startet: Option<DateTime<Utc>>,
    periode_avsluttet: Option<DateTime<Utc>>,
}

impl TestData {
    fn gen_test_data() -> Vec<TestData> {
        vec![TestData {
            key: -1,
            arbeidssoeker_id: 1,
            aktor_id: "101701234500",
            identitetsnummer: "01017012345",
            fornavn: "Ola",
            etternavn: "Nordmann",
            periode_id: Uuid::from_str("07206f7d-0323-483f-961b-bd571ff384df").unwrap(),
            periode_startet: Some(datetime_rfc3339("2026-02-01T12:00:00Z")),
            periode_avsluttet: Some(datetime_rfc3339("2026-01-31T12:00:00Z")),
        }]
    }
}

struct TestKafkaProducer {
    producer: FutureProducer,
    avro_serializer: AvroSerializer,
}

impl TestKafkaProducer {
    fn new() -> anyhow::Result<Self> {
        let kafka_config = read_kafka_config()?;
        let config = kafka_config.rdkafka_client_config()?;
        let producer: FutureProducer = config.create()?;
        let schema_registry_settings = create_schema_registry_settings()?;
        let avro_serializer = AvroSerializer::new(schema_registry_settings.clone());
        Ok(Self {
            producer,
            avro_serializer,
        })
    }

    #[allow(unused)]
    async fn send_start_perioder(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message =
                create_dummy_start_periode(d.identitetsnummer, d.periode_id, d.periode_startet);
            println!("Sender melding ({}): {:?}", Utc::now(), message);
            self.send_paw_avro_messages(
                "paw.arbeidssokerperioder-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_avslutt_perioder(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            if d.periode_avsluttet.is_some() {
                let message = create_dummy_avslutt_periode(
                    d.identitetsnummer,
                    d.periode_id,
                    d.periode_startet,
                    d.periode_avsluttet,
                );
                println!("Sender melding ({}): {:?}", Utc::now(), message);
                self.send_paw_avro_messages(
                    "paw.arbeidssokerperioder-v1",
                    Some(&d.key.to_be_bytes().to_vec()),
                    Some(message),
                )
                .await?;
            }
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_paw_avro_messages(
        &self,
        topic: &str,
        key: Option<&Vec<u8>>,
        message: Option<impl Serialize>,
    ) -> anyhow::Result<()> {
        match message {
            Some(m) => {
                let naming_strategy =
                    SubjectNameStrategy::TopicNameStrategy(topic.to_string(), false);
                let payload = self
                    .avro_serializer
                    .serialize(m, &naming_strategy)
                    .await
                    .expect("Failed to serialize message");
                let record = match key {
                    Some(k) => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic)
                        .key(k)
                        .payload(&payload),
                    None => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic).payload(&payload),
                };
                self.send_message(record).await
            }
            None => {
                let record = match key {
                    Some(k) => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic).key(k),
                    None => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic),
                };
                self.send_message(record).await
            }
        };

        Ok(())
    }

    #[allow(unused)]
    async fn send_json_messages(
        &self,
        topic: &str,
        key: Option<&Vec<u8>>,
        message: Option<impl Serialize>,
    ) -> anyhow::Result<()> {
        match message {
            Some(m) => {
                let payload = serde_json::to_vec(&m)?;
                let record = match key {
                    Some(k) => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic)
                        .key(k)
                        .payload(&payload),
                    None => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic).payload(&payload),
                };
                self.send_message(record).await
            }
            None => {
                let record = match key {
                    Some(k) => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic).key(k),
                    None => FutureRecord::<Vec<u8>, Vec<u8>>::to(topic),
                };
                self.send_message(record).await
            }
        };

        Ok(())
    }

    #[allow(unused)]
    async fn send_message<K, P>(&self, record: FutureRecord<'_, K, P>) -> Delivery
    where
        K: ToBytes + ?Sized,
        P: ToBytes + ?Sized,
    {
        self.producer
            .send(record, Duration::ZERO)
            .await
            .expect("Failed to send message")
    }
}
