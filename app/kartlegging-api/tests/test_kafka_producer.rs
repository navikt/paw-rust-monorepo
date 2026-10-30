use chrono::{DateTime, Utc};
use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
use kartlegging_api::config::read_kafka_config;
use nais_schema_registry::config::create_schema_registry_settings;
use paw_avro_serde::serde::AvroSerializer;
use pdl_hendelser::leesah::Endringstype;
use rdkafka::message::ToBytes;
use rdkafka::producer::future_producer::Delivery;
use rdkafka::producer::{FutureProducer, FutureRecord};
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use serde::Serialize;
use std::str::FromStr;
use std::time::Duration;
use test_data_generator::dab_oppfolgingsperiode::create_dummy_start_oppfolgingsperiode;
use test_data_generator::eksterne_hendelser::{
    create_dummy_avslutt_periode, create_dummy_bekreftelse, create_dummy_egenvurdering,
    create_dummy_opplysninger, create_dummy_profilering, create_dummy_start_paavegneav,
    create_dummy_start_periode, create_dummy_stopp_paavegneav, datetime_rfc3339,
};
use test_data_generator::pdl_hendelser::{create_dummy_navn, create_dummy_personhendelse_navn};
use uuid::Uuid;

#[ignore]
#[tokio::test]
async fn test_send_messages() -> anyhow::Result<()> {
    let data = TestData::gen_test_data();
    let producer = TestKafkaProducer::new()?;

    // producer.send_start_perioder(&data).await?;
    //
    // producer.send_opplysninger(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
    // producer.send_profileringer(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
    // producer.send_egenvurderinger(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
    // producer.send_bekreftelser(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
    // producer.send_start_paavegneav(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
    // producer.send_start_oppfolgingsperioder(&data).await?;
    //
    // tokio::time::sleep(Duration::from_millis(100)).await;
    //
    // producer.send_stopp_paavegneav(&data).await?;
    //
    // producer.send_avslutt_perioder(&data).await?;
    //
    producer.send_pdl_leesah_personhendelse(&data).await?;

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
    opplysninger_id: Uuid,
    profilering_id: Uuid,
    egenvurdering_id: Uuid,
    bekreftelse_id: Uuid,
    bekreftelse_gjelder_fra: Option<DateTime<Utc>>,
    bekreftelse_gjelder_til: Option<DateTime<Utc>>,
    oppfolgingsperiode_id: Uuid,
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
            opplysninger_id: Uuid::from_str("e1c3d0e2-4b7b-4f1a-ae3b-2f5c6d7e8f9a").unwrap(),
            profilering_id: Uuid::from_str("da5f8f47-0a48-4553-98b6-aa4afa9cb059").unwrap(),
            egenvurdering_id: Uuid::from_str("c3d0e2e1-4b7b-4f1a-ae3b-2f5c6d7e8f9a").unwrap(),
            bekreftelse_id: Uuid::from_str("8b9311e6-ab49-41fa-adb9-e5a743370cdc").unwrap(),
            bekreftelse_gjelder_fra: Some(datetime_rfc3339("2025-12-16T12:00:00Z")),
            bekreftelse_gjelder_til: Some(datetime_rfc3339("2025-12-31T12:00:00Z")),
            oppfolgingsperiode_id: Uuid::from_str("6c34d105-b9cd-471f-b2b4-2812466f1c66").unwrap(),
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
    async fn send_opplysninger(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message =
                create_dummy_opplysninger(d.identitetsnummer, d.periode_id, d.opplysninger_id);
            println!("Sender melding ({}): {:?}", Utc::now(), message);
            self.send_paw_avro_messages(
                "paw.opplysninger-om-arbeidssoeker-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_profileringer(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message = create_dummy_profilering(
                d.identitetsnummer,
                d.periode_id,
                d.opplysninger_id,
                d.profilering_id,
            );
            println!("Sender melding ({}): {:?}", Utc::now(), message);
            self.send_paw_avro_messages(
                "paw.arbeidssoker-profilering-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_egenvurderinger(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message = create_dummy_egenvurdering(
                d.identitetsnummer,
                d.periode_id,
                d.profilering_id,
                d.egenvurdering_id,
            );
            println!(
                "Sender melding timestamp: {} key: {}, value: {:?}",
                Utc::now(),
                d.key,
                message
            );
            self.send_paw_avro_messages(
                "paw.arbeidssoeker-egenvurdering-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_bekreftelser(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message = create_dummy_bekreftelse(
                d.identitetsnummer,
                d.periode_id,
                d.bekreftelse_id,
                d.bekreftelse_gjelder_fra,
                d.bekreftelse_gjelder_til,
                false,
                true,
            );
            println!(
                "Sender melding timestamp: {} key: {}, value: {:?}",
                Utc::now(),
                d.key,
                message
            );
            self.send_paw_avro_messages(
                "paw.arbeidssoker-bekreftelse-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_start_paavegneav(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message = create_dummy_start_paavegneav(
                d.periode_id,
                Bekreftelsesloesning::Arbeidssoekerregisteret,
            );
            println!(
                "Sender melding timestamp: {} key: {}, value: {:?}",
                Utc::now(),
                d.key,
                message
            );
            self.send_paw_avro_messages(
                "paw.arbeidssoker-bekreftelse-paavegneav-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_stopp_paavegneav(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message = create_dummy_stopp_paavegneav(
                d.periode_id,
                Bekreftelsesloesning::Arbeidssoekerregisteret,
            );
            println!(
                "Sender melding timestamp: {} key: {}, value: {:?}",
                Utc::now(),
                d.key,
                message
            );
            self.send_paw_avro_messages(
                "paw.arbeidssoker-bekreftelse-paavegneav-v1",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_start_oppfolgingsperioder(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let message = create_dummy_start_oppfolgingsperiode(
                d.oppfolgingsperiode_id,
                d.aktor_id,
                d.identitetsnummer,
                "1234",
            );
            println!(
                "Sender melding timestamp: {} key: {}, value: {:?}",
                Utc::now(),
                d.key,
                message
            );
            self.send_json_messages(
                "poao.siste-oppfolgingsperiode-v3",
                Some(&d.key.to_be_bytes().to_vec()),
                Some(message),
            )
            .await?;
        }

        Ok(())
    }

    #[allow(unused)]
    async fn send_pdl_leesah_personhendelse(&self, data: &Vec<TestData>) -> anyhow::Result<()> {
        for d in data {
            let navn = create_dummy_navn(d.fornavn, None, d.etternavn);
            let message = create_dummy_personhendelse_navn(
                vec![d.identitetsnummer],
                Endringstype::Opprettet,
                Some(navn),
            );
            println!(
                "Sender melding timestamp: {} key: {}, value: {:?}",
                Utc::now(),
                d.aktor_id,
                message
            );
            let topic = "pdl.leesah-v1";
            let key_strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), true);
            let value_strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), false);
            let key = self
                .avro_serializer
                .serialize(d.aktor_id, &key_strategy)
                .await?;
            let payload = self
                .avro_serializer
                .serialize(&message, &value_strategy)
                .await?;
            let record = FutureRecord::to(topic).key(&key).payload(&payload);
            self.send_message(record).await;
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
