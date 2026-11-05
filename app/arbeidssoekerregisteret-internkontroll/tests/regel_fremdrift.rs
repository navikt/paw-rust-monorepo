mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    batch::{BatchRad, neste_batch},
    fremdrift::{Fremdrift, hent_fremdrift, oppdater_fremdrift},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};
use common::TestApp;
use common::melding::Melding;
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::StartetBuilder;
use sqlx::PgConnection;
use test_data_generator::eksterne_hendelser::create_dummy_start_periode;
use uuid::Uuid;

const REGEL: RegelId = RegelId::IngenPeriodeUtenStartet;
const V1: Regelversjon = Regelversjon::new(1);

fn periode() -> eksterne_hendelser::periode::Periode {
    create_dummy_start_periode("12345678901", Uuid::new_v4(), None)
}

/// Fremdrift for `topics`, lagret eller -1, slik regelkjøringen bygger den.
async fn fremdrift(conn: &mut PgConnection, topics: &[i16]) -> Vec<Fremdrift> {
    let lagret = hent_fremdrift(conn, REGEL, V1, 0).await.unwrap();
    topics
        .iter()
        .map(|&topic_id| {
            lagret
                .iter()
                .find(|f| f.topic_id == topic_id)
                .copied()
                .unwrap_or(Fremdrift {
                    regel: REGEL,
                    regelversjon: V1,
                    topic_id,
                    topic_partition: 0,
                    siste_offset: -1,
                    siste_stream_ms: None,
                })
        })
        .collect()
}

/// Flytter fremdriften til siste rad per topic i batchen og lagrer den.
async fn lagre(conn: &mut PgConnection, mut fremdrift: Vec<Fremdrift>, batch: &[BatchRad]) {
    for rad in batch {
        let f = fremdrift
            .iter_mut()
            .find(|f| f.topic_id == rad.topic_id)
            .unwrap();
        f.siste_offset = rad.record_offset;
        f.siste_stream_ms = rad.stream_ms;
    }
    oppdater_fremdrift(conn, &fremdrift).await.unwrap();
}

fn posisjoner(batch: &[BatchRad]) -> Vec<(i16, i64, Option<i64>)> {
    batch
        .iter()
        .map(|r| (r.topic_id, r.record_offset, r.stream_ms))
        .collect()
}

#[tokio::test]
async fn batch_tar_eldste_stream_time_foerst_og_hopper_aldri_over() {
    let app = TestApp::start().await;
    let (nyere, eldre) = (
        app.topic_id(&Topic::Hendelselogg),
        app.topic_id(&Topic::Periode),
    );
    let topics = [nyere, eldre];
    let mut conn = app.pool.acquire().await.unwrap();

    // Offset 2 kommer etter stream_time, så stream_ms blir 2_000.
    for (offset, ts) in [(0, 1_000), (1, 2_000), (2, 1_500)] {
        app.les_inn(
            Melding::ny(StartetBuilder::default())
                .offset(offset)
                .tidspunkt_ms(ts),
        )
        .await;
    }
    let f = fremdrift(&mut conn, &topics).await;
    let batch = neste_batch(&mut conn, 0, &f, 10).await.unwrap();
    assert_eq!(
        posisjoner(&batch),
        [
            (nyere, 0, Some(1_000)),
            (nyere, 1, Some(2_000)),
            (nyere, 2, Some(2_000))
        ]
    );
    lagre(&mut conn, f, &batch).await;

    // Eldre topic leses inn etter at nyere har fremdrift lenger fram i tid.
    for (offset, ts) in [(0, 100), (1, 200), (2, 300)] {
        app.les_inn(Melding::ny(periode()).offset(offset).tidspunkt_ms(ts))
            .await;
    }
    let f = fremdrift(&mut conn, &topics).await;
    let batch = neste_batch(&mut conn, 0, &f, 2).await.unwrap();
    assert_eq!(
        posisjoner(&batch),
        [(eldre, 0, Some(100)), (eldre, 1, Some(200))]
    );
    lagre(&mut conn, f, &batch).await;

    let f = fremdrift(&mut conn, &topics).await;
    let batch = neste_batch(&mut conn, 0, &f, 2).await.unwrap();
    assert_eq!(posisjoner(&batch), [(eldre, 2, Some(300))]);
    lagre(&mut conn, f, &batch).await;

    let f = fremdrift(&mut conn, &topics).await;
    assert!(neste_batch(&mut conn, 0, &f, 2).await.unwrap().is_empty());
    let siste: Vec<_> = f.iter().map(|f| (f.topic_id, f.siste_offset)).collect();
    assert_eq!(siste, [(nyere, 2), (eldre, 2)]);
}
