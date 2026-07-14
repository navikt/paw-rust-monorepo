use dab_oppfolgingperioder::oppfolgingsperiode::POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC;
use eksterne_hendelser::bekreftelse::bekreftelse::PAW_BEKREFTELSE_TOPIC;
use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
use eksterne_hendelser::egenvurdering::PAW_EGENVURDERING_TOPIC;
use eksterne_hendelser::opplysninger::PAW_OPPLYSNINGER_TOPIC;
use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
use eksterne_hendelser::profilering::PAW_PROFILERING_TOPIC;
use eksterne_hendelser::serde::AvroSerializer;
use kartlegging_api::config::read_kafka_config;
use nais_schema_registry::config::create_schema_registry_settings;
use rdkafka::producer::{FutureProducer, FutureRecord};
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use serde::Serialize;
use std::str::FromStr;
use std::time::Duration;
use test_data_generator::dab_oppfolgingsperiode::create_dummy_startet_oppfolgingsperiode;
use test_data_generator::eksterne_hendelser::{
    create_dummy_bekreftelse, create_dummy_egenvurdering, create_dummy_opplysninger,
    create_dummy_paavegneav_start, create_dummy_profilering, create_dummy_startet_periode,
};
use uuid::Uuid;

struct Ids {
    aktor_id: &'static str,
    identitetsnummer: &'static str,
    periode_id: Uuid,
    opplysninger_id: Uuid,
    profilering_id: Uuid,
    egenvurdering_id: Uuid,
    bekreftelse_id: Uuid,
    oppfolgingsperiode_id: Uuid,
}

#[ignore]
#[tokio::test]
async fn test_send_messages() -> anyhow::Result<()> {
    let kafka_config = read_kafka_config()?;
    let config = kafka_config.rdkafka_client_config()?;
    let producer: FutureProducer = config.create()?;
    let schema_registry_settings = create_schema_registry_settings()?;
    let serializer = AvroSerializer::new(schema_registry_settings);

    let ids = gen_ids();

    for id in &ids {
        let message = create_dummy_startet_periode(id.identitetsnummer, id.periode_id);
        println!("Sender melding: {:?}", message);
        send_avro_messages(&producer, &serializer, PAW_PERIODE_TOPIC, message).await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    for id in &ids {
        let message =
            create_dummy_opplysninger(id.identitetsnummer, id.periode_id, id.opplysninger_id);
        println!("Sender melding: {:?}", message);
        send_avro_messages(&producer, &serializer, PAW_OPPLYSNINGER_TOPIC, message).await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    for id in &ids {
        let message = create_dummy_profilering(
            id.identitetsnummer,
            id.periode_id,
            id.opplysninger_id,
            id.profilering_id,
        );
        println!("Sender melding: {:?}", message);
        send_avro_messages(&producer, &serializer, PAW_PROFILERING_TOPIC, message).await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    for id in &ids {
        let message = create_dummy_egenvurdering(
            id.identitetsnummer,
            id.periode_id,
            id.profilering_id,
            id.egenvurdering_id,
        );
        println!("Sender melding: {:?}", message);
        send_avro_messages(&producer, &serializer, PAW_EGENVURDERING_TOPIC, message).await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    for id in &ids {
        let message =
            create_dummy_bekreftelse(id.identitetsnummer, id.periode_id, id.bekreftelse_id);
        println!("Sender melding: {:?}", message);
        send_avro_messages(&producer, &serializer, PAW_BEKREFTELSE_TOPIC, message).await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    for id in &ids {
        let message = create_dummy_paavegneav_start(id.periode_id);
        println!("Sender melding: {:?}", message);
        send_avro_messages(
            &producer,
            &serializer,
            PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC,
            message,
        )
        .await?;
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    for id in &ids {
        let message = create_dummy_startet_oppfolgingsperiode(
            id.oppfolgingsperiode_id,
            id.aktor_id,
            id.identitetsnummer,
            "1234",
        );
        println!("Sender melding: {:?}", message);
        send_json_messages(&producer, POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC, message).await?;
    }

    Ok(())
}

fn gen_ids() -> Vec<Ids> {
    vec![Ids {
        aktor_id: "101701234500",
        identitetsnummer: "01017012345",
        periode_id: Uuid::from_str("16b58697-131f-4715-9559-40a0644158f6").unwrap(),
        opplysninger_id: Uuid::from_str("e1c3d0e2-4b7b-4f1a-ae3b-2f5c6d7e8f9a").unwrap(),
        profilering_id: Uuid::from_str("da5f8f47-0a48-4553-98b6-aa4afa9cb059").unwrap(),
        egenvurdering_id: Uuid::from_str("c3d0e2e1-4b7b-4f1a-ae3b-2f5c6d7e8f9a").unwrap(),
        bekreftelse_id: Uuid::from_str("e1c3d0e2-4b7b-4f1a-ae3b-2f5c6d7e8f9a").unwrap(),
        oppfolgingsperiode_id: Uuid::from_str("6c34d105-b9cd-471f-b2b4-2812466f1c66").unwrap(),
    }]
}

async fn send_avro_messages(
    producer: &FutureProducer,
    serializer: &AvroSerializer,
    topic: &str,
    message: impl Serialize,
) -> anyhow::Result<()> {
    let naming_strategy = SubjectNameStrategy::TopicNameStrategy(topic.to_string(), false);
    let payload = serializer.serialize(message, &naming_strategy).await?;
    producer
        .send(
            FutureRecord::to(topic)
                .payload(&payload)
                .key(&1i64.to_be_bytes()),
            Duration::ZERO,
        )
        .await
        .map_err(|(e, _)| anyhow::anyhow!(e))?;

    Ok(())
}

async fn send_json_messages(
    producer: &FutureProducer,
    topic: &str,
    message: impl Serialize,
) -> anyhow::Result<()> {
    let payload = serde_json::to_vec(&message)?;
    producer
        .send(
            FutureRecord::to(topic)
                .payload(&payload)
                .key(&1i64.to_be_bytes()),
            Duration::ZERO,
        )
        .await
        .map_err(|(e, _)| anyhow::anyhow!(e))?;

    Ok(())
}
