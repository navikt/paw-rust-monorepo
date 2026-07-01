use chrono::{Duration, Utc};
use eksterne_hendelser::bekreftelse::bekreftelse::{Bekreftelse, BEKREFTELSE_TOPIC};
use eksterne_hendelser::bekreftelse::bekreftelsesloesning::Bekreftelsesloesning;
use eksterne_hendelser::bekreftelse::svar::Svar;
use eksterne_hendelser::bruker::Bruker;
use eksterne_hendelser::brukertype::BrukerType;
use eksterne_hendelser::metadata::Metadata;
use eksterne_hendelser::periode::{Periode, PERIODE_TOPIC};
use eksterne_hendelser::serde::AvroSerializer;
use kartlegging_api::config::read_kafka_config;
use nais_schema_registry::config::create_schema_registry_settings;
use rdkafka::producer::{FutureProducer, FutureRecord};
use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
use std::str::FromStr;
use uuid::Uuid;

#[ignore]
#[tokio::test]
async fn test_send_messages() -> anyhow::Result<()> {
    let kafka_config = read_kafka_config()?;
    let config = kafka_config.rdkafka_client_config()?;
    let producer: FutureProducer = config.create()?;
    let schema_registry_settings = create_schema_registry_settings()?;
    let serializer = AvroSerializer::new(schema_registry_settings);

    let _ = send_dummy_perioder(&producer, &serializer).await?;

    Ok(())
}

async fn send_dummy_perioder(
    producer: &FutureProducer,
    serializer: &AvroSerializer,
) -> anyhow::Result<Vec<Periode>> {
    let naming_strategy = SubjectNameStrategy::TopicNameStrategy(PERIODE_TOPIC.to_string(), false);
    let messages = vec![Periode {
        id: Uuid::from_str("16b58697-131f-4715-9559-40a0644158f6")?,
        identitetsnummer: "01017012345".to_string(),
        startet: Metadata {
            tidspunkt: Utc::now(),
            utfoert_av: Bruker {
                bruker_type: BrukerType::Sluttbruker,
                id: "01017012345".to_string(),
                sikkerhetsnivaa: Some("tokenx:Level4".to_string()),
            },
            kilde: "testing".to_string(),
            aarsak: "Test".to_string(),
            tidspunkt_fra_kilde: None,
        },
        avsluttet: None,
    }];
    for message in &messages {
        let payload = serializer.serialize(message, &naming_strategy).await?;
        producer
            .send(
                FutureRecord::to(PERIODE_TOPIC)
                    .payload(&payload)
                    .key(&1i64.to_be_bytes()),
                std::time::Duration::ZERO,
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!(e))?;
    }

    Ok(messages)
}

async fn send_dummy_bekreftelser(
    producer: &FutureProducer,
    serializer: &AvroSerializer,
    perioder: Vec<Periode>,
) -> anyhow::Result<()> {
    let naming_strategy =
        SubjectNameStrategy::TopicNameStrategy(BEKREFTELSE_TOPIC.to_string(), false);
    let messages = vec![Bekreftelse {
        id: Uuid::from_str("da5f8f47-0a48-4553-98b6-aa4afa9cb059")?,
        periode_id: Uuid::from_str("16b58697-131f-4715-9559-40a0644158f6")?,
        bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
        svar: Svar {
            sendt_inn_av: Metadata {
                tidspunkt: Utc::now(),
                utfoert_av: Bruker {
                    bruker_type: BrukerType::Sluttbruker,
                    id: "01017012345".to_string(),
                    sikkerhetsnivaa: None,
                },
                kilde: "testing".to_string(),
                aarsak: "Test".to_string(),
                tidspunkt_fra_kilde: None,
            },
            gjelder_fra: Utc::now() - Duration::days(6),
            gjelder_til: Utc::now() - Duration::days(20),
            har_jobbet_i_denne_perioden: false,
            vil_fortsette_som_arbeidssoeker: true,
        },
    }];
    for message in messages {
        let payload = serializer.serialize(&message, &naming_strategy).await?;
        producer
            .send(
                FutureRecord::to(BEKREFTELSE_TOPIC)
                    .payload(&payload)
                    .key(&1i64.to_be_bytes()),
                std::time::Duration::ZERO,
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!(e))?;
    }

    Ok(())
}
