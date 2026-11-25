//! Innlesing ende til ende: melding inn, record_metadata og eventuell backup i databasen.
//! Oppsettet speiler app_runner, slik at topics, dekodere og backup-krav er de ekte.

mod common;

use arbeidssoekerregisteret_internkontroll::features::innlesing::record::kafka_record_oppslag::hent_kafka_record;
use common::melding::Melding;
use common::{TestApp, topic_navn};
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::StartetBuilder;
use rdkafka::message::OwnedMessage;
use rdkafka::{Message, Timestamp};
use test_data_generator::eksterne_hendelser::create_dummy_start_periode;
use uuid::Uuid;

const TRACE: &str = "0af7651916cd43dd8448eb211c80319c";

#[tokio::test]
async fn meldinger_lagres_med_metadata_og_backup_etter_topic() {
    let app = TestApp::start().await;
    let pool = &app.pool;

    let startet = StartetBuilder {
        arbeidssoeker_id: 42,
        ..Default::default()
    };
    let periode_id = startet.hendelse_id;
    let startet = app
        .les_inn(
            Melding::ny(startet)
                .offset(0)
                .tidspunkt_ms(1_000)
                .header("traceparent", format!("00-{TRACE}-b7ad6b7169203331-01")),
        )
        .await;
    let ugyldig = OwnedMessage::new(
        Some(b"ikke json".to_vec()),
        None,
        topic_navn(&Topic::Hendelselogg).into(),
        Timestamp::CreateTime(1_100),
        0,
        1,
        None,
    );
    app.les_inn_raa(&ugyldig).await;
    let periode = create_dummy_start_periode("12345678901", periode_id, None);
    app.les_inn(Melding::ny(periode).offset(2).tidspunkt_ms(1_200))
        .await;

    type Rad = (
        i64,
        String,
        String,
        Option<String>,
        Option<Uuid>,
        Option<i64>,
        Option<Uuid>,
        bool,
    );
    let rader: Vec<Rad> = sqlx::query_as(
        "SELECT m.record_offset, m.signature_status::text, m.payload_status::text, m.meldingstype::text,
                m.periode_id, m.arbeidssoeker_id, m.trace_id, r.metadata_id IS NOT NULL
         FROM record_metadata m LEFT JOIN kafka_record r ON r.metadata_id = m.id
         ORDER BY m.record_offset",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    let usignert = || "unsigned_before_required".to_string();
    assert_eq!(
        rader,
        vec![
            (
                0,
                usignert(),
                "ok".into(),
                Some("hendelse_startet".into()),
                Some(periode_id),
                Some(42),
                Uuid::parse_str(TRACE).ok(),
                true
            ),
            (
                1,
                usignert(),
                "ugyldig".into(),
                None,
                None,
                None,
                None,
                true
            ),
            (
                2,
                usignert(),
                "ok".into(),
                Some("periode_startet".into()),
                Some(periode_id),
                None,
                None,
                false
            ),
        ]
    );

    let mut conn = pool.acquire().await.unwrap();
    let lagret = hent_kafka_record(&mut conn, app.topic_id(&Topic::Hendelselogg), 0, 0)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(lagret.payload(), startet.payload());
    assert_eq!(
        lagret.headers().map(rdkafka::message::Headers::count),
        Some(1)
    );
}

type Person = (Option<String>, Option<i64>, Option<String>, Option<String>);

async fn person(app: &TestApp, topic: &Topic) -> Person {
    sqlx::query_as(
        "SELECT identitetsnummer, utfoert_tidspunkt_ms, utfoert_av_type::text, utfoert_av_id
         FROM record_metadata WHERE topic_id = $1",
    )
    .bind(app.topic_id(topic))
    .fetch_one(&app.pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn hvem_det_gjelder_og_hvem_som_utfoerte_lagres_i_millisekunder() {
    let app = TestApp::start().await;
    app.les_inn(StartetBuilder {
        identitetsnummer: "11111111111".into(),
        utfoert_av_id: "Z123456".into(),
        bruker_type: interne_hendelser::vo::BrukerType::Veileder,
        tidspunkt: paw_test::hendelse_builder::rfc3339("2024-01-01T12:00:00.123Z"),
        ..Default::default()
    })
    .await;
    app.les_inn(create_dummy_start_periode(
        "22222222222",
        Uuid::new_v4(),
        Some(paw_test::hendelse_builder::rfc3339(
            "2024-01-01T12:00:00.456Z",
        )),
    ))
    .await;

    assert_eq!(
        person(&app, &Topic::Hendelselogg).await,
        (
            Some("11111111111".into()),
            Some(1_704_110_400_123),
            Some("veileder".into()),
            Some("Z123456".into())
        )
    );
    let (identitetsnummer, tidspunkt_ms, _, _) = person(&app, &Topic::Periode).await;
    assert_eq!(identitetsnummer.as_deref(), Some("22222222222"));
    assert_eq!(tidspunkt_ms, Some(1_704_110_400_456));
}
