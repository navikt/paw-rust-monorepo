//! IngenPeriodeUtenStartet: en periode uten startet-hendelse gir funn, som
//! fjernes hvis startet-hendelsen kommer senere.

mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::ingen_periode_uten_startet::IngenPeriodeUtenStartet;
use common::{TestApp, melding::Melding, topic_navn};
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::StartetBuilder;
use test_data_generator::eksterne_hendelser::create_dummy_start_periode;
use uuid::Uuid;

const T: i64 = 1_704_110_400_000;

fn startet(periode_id: Uuid, tid: i64) -> Melding<StartetBuilder> {
    Melding::ny(StartetBuilder {
        hendelse_id: periode_id,
        ..Default::default()
    })
    .tidspunkt_ms(tid)
}

fn periode(periode_id: Uuid, tid: i64) -> Melding<eksterne_hendelser::periode::Periode> {
    Melding::ny(create_dummy_start_periode("12345678901", periode_id, None)).tidspunkt_ms(tid)
}

fn regel() -> Vec<Box<dyn arbeidssoekerregisteret_internkontroll::features::regler::regel::Regel>> {
    vec![Box::new(IngenPeriodeUtenStartet)]
}

#[tokio::test]
async fn periode_med_startet_gir_ikke_funn() {
    let app = TestApp::start().await;
    let id = Uuid::new_v4();
    app.les_inn(startet(id, T)).await;
    app.les_inn(periode(id, T + 10)).await;

    app.kjor_regler(regel()).await;

    assert!(app.funn().await.is_empty());
}

#[tokio::test]
async fn periode_uten_startet_gir_funn() {
    let app = TestApp::start().await;
    app.les_inn(startet(Uuid::new_v4(), T)).await;
    app.les_inn(periode(Uuid::new_v4(), T + 10)).await;

    app.kjor_regler(regel()).await;

    assert_eq!(
        app.funn().await,
        [(topic_navn(&Topic::Periode).to_string(), 0, 1)]
    );
}

#[tokio::test]
async fn startet_som_kommer_etter_perioden_fjerner_funnet() {
    let app = TestApp::start().await;
    let id = Uuid::new_v4();
    app.les_inn(periode(id, T)).await;
    app.kjor_regler(regel()).await;
    assert_eq!(app.funn().await.len(), 1);

    app.les_inn(startet(id, T + 10)).await;
    app.kjor_regler(regel()).await;

    assert!(app.funn().await.is_empty());
}
