//! StartetGirPeriode: en startet-hendelse som skulle startet en periode, men
//! ikke har fått en, gir funn. Gjentatte forsøk mens perioden er åpen er i orden.

mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    regel::Regel, startet_gir_periode::StartetGirPeriode,
};
use common::{TestApp, melding::Melding, topic_navn};
use eksterne_hendelser::periode::Periode;
use interne_hendelser::Avsluttet;
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

fn avsluttet(periode_id: Uuid, tid: i64) -> Melding<Avsluttet> {
    let s = StartetBuilder::default().build();
    Melding::ny(Avsluttet {
        hendelse_id: Uuid::new_v4(),
        id: s.id,
        identitetsnummer: s.identitetsnummer,
        metadata: s.metadata,
        opplysninger: Default::default(),
        periode_id: Some(periode_id),
        aarsaks_informasjon: None,
    })
    .tidspunkt_ms(tid)
}

fn periode(periode_id: Uuid, tid: i64) -> Melding<Periode> {
    Melding::ny(create_dummy_start_periode("12345678901", periode_id, None)).tidspunkt_ms(tid)
}

fn regel() -> Vec<Box<dyn Regel>> {
    vec![Box::new(StartetGirPeriode)]
}

fn hendelselogg(offset: i64) -> (String, i64, i16) {
    (topic_navn(&Topic::Hendelselogg).to_string(), offset, 1)
}

#[tokio::test]
async fn startet_med_periode_gir_ikke_funn() {
    let app = TestApp::start().await;
    let id = Uuid::new_v4();
    app.les_inn(periode(id, T)).await;
    app.les_inn(startet(id, T + 10)).await;

    app.kjor_regler(regel()).await;

    assert!(app.funn().await.is_empty());
}

#[tokio::test]
async fn startet_uten_periode_gir_funn_som_fjernes_naar_perioden_kommer() {
    let app = TestApp::start().await;
    let id = Uuid::new_v4();
    app.les_inn(startet(id, T)).await;
    app.kjor_regler(regel()).await;
    assert_eq!(app.funn().await, [hendelselogg(0)]);

    app.les_inn(periode(id, T + 10)).await;
    app.kjor_regler(regel()).await;

    assert!(app.funn().await.is_empty());
}

#[tokio::test]
async fn nytt_forsoek_mens_perioden_er_aapen_gir_ikke_funn() {
    let app = TestApp::start().await;
    let id = Uuid::new_v4();
    app.les_inn(startet(id, T)).await;
    app.les_inn(periode(id, T + 10)).await;
    app.les_inn(startet(Uuid::new_v4(), T + 20)).await;

    app.kjor_regler(regel()).await;

    assert!(app.funn().await.is_empty());
}

#[tokio::test]
async fn startet_etter_avsluttet_uten_periode_gir_funn() {
    let app = TestApp::start().await;
    let id = Uuid::new_v4();
    app.les_inn(startet(id, T)).await;
    app.les_inn(periode(id, T + 10)).await;
    app.les_inn(avsluttet(id, T + 20)).await;
    app.les_inn(startet(Uuid::new_v4(), T + 30)).await;

    app.kjor_regler(regel()).await;

    assert_eq!(app.funn().await, [hendelselogg(2)]);
}

#[tokio::test]
async fn bare_foerste_startet_uten_periode_gir_funn() {
    let app = TestApp::start().await;
    app.les_inn(startet(Uuid::new_v4(), T)).await;
    app.les_inn(startet(Uuid::new_v4(), T + 10)).await;

    app.kjor_regler(regel()).await;

    assert_eq!(app.funn().await, [hendelselogg(0)]);
}
