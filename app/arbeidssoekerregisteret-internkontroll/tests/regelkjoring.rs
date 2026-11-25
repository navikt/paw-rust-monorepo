//! Regelkjøring ende til ende med StartetErSignert:
//! meldinger leses inn, regelkjøringen går runder til det ikke er mer å gjøre.

mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    kjoring::kjor_runde,
    regel::{RegistrertRegel, registrer_regler},
    startet_er_signert::StartetErSignert,
};
use common::TestApp;
use common::melding::Melding;
use paw_kafka::hwm::rebalance::topic_partition_update::TopicPartition;
use paw_kafka::hwm::stream::assigned_partitions::AssignedPartitions;
use paw_kafka::topics::Topic;
use paw_rust_base::env::RuntimeEnv;
use paw_test::hendelse_builder::StartetBuilder;
use rdkafka::Message;

use common::topic_navn;

fn regler(app: &TestApp) -> Vec<RegistrertRegel> {
    registrer_regler(
        vec![Box::new(StartetErSignert)],
        &app.topic_ids,
        &RuntimeEnv::Local,
    )
}

fn tildel(tildelt: &AssignedPartitions, partisjon: i32) {
    tildelt.add([TopicPartition {
        topic: topic_navn(&Topic::Hendelselogg).to_string(),
        partition: partisjon,
    }]);
}

async fn kjor_til_tom(app: &TestApp, regler: &[RegistrertRegel], tildelt: &AssignedPartitions) {
    while kjor_runde(&app.pool, regler, tildelt, 2).await.unwrap() > 0 {}
}

/// (partisjon, offset) for meldinger med funn, sortert.
async fn funn(app: &TestApp) -> Vec<(i32, i64)> {
    sqlx::query_as(
        "SELECT rm.record_partition, rm.record_offset
         FROM funn f JOIN record_metadata rm ON rm.id = f.record_metadata_id
         ORDER BY 1, 2",
    )
    .fetch_all(&app.pool)
    .await
    .unwrap()
}

async fn les_inn(app: &TestApp, partisjon: i32, signert: bool) {
    let msg = app
        .les_inn(Melding::ny(StartetBuilder::default()).partisjon(partisjon))
        .await;
    if !signert {
        sqlx::query(
            "UPDATE record_metadata SET signature_status = 'missing_signature'
             WHERE record_partition = $1 AND record_offset = $2",
        )
        .bind(partisjon)
        .bind(msg.offset())
        .execute(&app.pool)
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn usignert_startet_gir_funn_over_flere_batcher() {
    let app = TestApp::start().await;
    let regler = regler(&app);
    let tildelt = AssignedPartitions::default();
    tildel(&tildelt, 0);
    for signert in [true, false, true, false, false] {
        les_inn(&app, 0, signert).await;
    }

    kjor_til_tom(&app, &regler, &tildelt).await;
    assert_eq!(funn(&app).await, [(0, 1), (0, 3), (0, 4)]);

    // Ny melding etter at fremdriften er lagret: bare den vurderes.
    les_inn(&app, 0, false).await;
    kjor_til_tom(&app, &regler, &tildelt).await;
    assert_eq!(funn(&app).await, [(0, 1), (0, 3), (0, 4), (0, 5)]);
}

#[tokio::test]
async fn partisjon_som_ikke_er_tildelt_blir_ikke_kjort() {
    let app = TestApp::start().await;
    let regler = regler(&app);
    let tildelt = AssignedPartitions::default();
    tildel(&tildelt, 0);
    les_inn(&app, 1, false).await;

    kjor_til_tom(&app, &regler, &tildelt).await;
    assert!(funn(&app).await.is_empty());

    tildel(&tildelt, 1);
    kjor_til_tom(&app, &regler, &tildelt).await;
    assert_eq!(funn(&app).await, [(1, 0)]);
}
