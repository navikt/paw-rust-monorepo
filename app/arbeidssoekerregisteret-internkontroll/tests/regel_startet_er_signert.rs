//! StartetErSignert: bare `hendelse_startet` med feil signatur gir funn.

mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::startet_er_signert::StartetErSignert;
use common::{TestApp, melding::Melding, topic_navn};
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::StartetBuilder;
use test_data_generator::eksterne_hendelser::create_dummy_start_periode;
use uuid::Uuid;

/// Setter signaturstatus etter innlesing. Ekte signering krever en privat nøkkel testene ikke har.
async fn sett_status(app: &TestApp, topic: &Topic, offset: i64, status: &str) {
    sqlx::query(
        "UPDATE record_metadata SET signature_status = $1::signature_status
         WHERE topic_id = $2 AND record_offset = $3",
    )
    .bind(status)
    .bind(app.topic_id(topic))
    .bind(offset)
    .execute(&app.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn startet_uten_signatur_gir_funn_andre_meldinger_ikke() {
    let app = TestApp::start().await;
    // Offset 0: usignert før kravet, i orden.
    app.les_inn(Melding::ny(StartetBuilder::default())).await;
    // Offset 1: mangler signatur etter kravet.
    app.les_inn(Melding::ny(StartetBuilder::default())).await;
    sett_status(&app, &Topic::Hendelselogg, 1, "missing_signature").await;
    // Periode uten signatur er ikke denne regelens sak.
    app.les_inn(create_dummy_start_periode(
        "12345678901",
        Uuid::new_v4(),
        None,
    ))
    .await;
    sett_status(&app, &Topic::Periode, 0, "missing_signature").await;

    app.kjor_regler(vec![Box::new(StartetErSignert)]).await;

    let hendelselogg = topic_navn(&Topic::Hendelselogg).to_string();
    assert_eq!(app.funn().await, [(hendelselogg, 1, 1)]);
}
