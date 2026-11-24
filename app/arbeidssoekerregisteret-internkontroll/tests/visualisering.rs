//! Visualiseringen mot ekte database: alle faner og filtre svarer, og
//! innholdet stemmer med det som er lest inn.

mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    kjoring::kjor_runde,
    regel::{alle_regler, registrer_regler},
};
use arbeidssoekerregisteret_internkontroll::features::visualisering::{VisState, routes};
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use common::{TestApp, melding::Melding, topic_navn};
use paw_kafka::hwm::rebalance::topic_partition_update::TopicPartition;
use paw_kafka::hwm::stream::assigned_partitions::AssignedPartitions;
use paw_kafka::topics::Topic;
use paw_rust_base::env::RuntimeEnv;
use paw_test::hendelse_builder::StartetBuilder;
use tower::ServiceExt;

async fn hent(app: &Router, sti: &str, htmx: bool) -> (StatusCode, String) {
    let mut req = Request::get(sti);
    if htmx {
        req = req.header("HX-Request", "true");
    }
    let response = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// Leser inn én melding uten trace, kjører regelen og gir den en signaturfeil.
async fn app_med_data() -> (TestApp, Router) {
    let app = TestApp::start().await;
    app.les_inn(Melding::ny(StartetBuilder::default())).await;
    let regler = registrer_regler(alle_regler(), &app.topic_ids, &RuntimeEnv::Local);
    let tildelt = AssignedPartitions::default();
    tildelt.add([TopicPartition {
        topic: topic_navn(&Topic::Hendelselogg).to_string(),
        partition: 0,
    }]);
    while kjor_runde(&app.pool, &regler, &tildelt, 10).await.unwrap() > 0 {}
    sqlx::query("UPDATE record_metadata SET signature_status = 'missing_signature'")
        .execute(&app.pool)
        .await
        .unwrap();
    let router = routes(
        VisState {
            pg_pool: app.pool.clone(),
            hwm_version: 1,
        },
        None,
    );
    (app, router)
}

#[tokio::test]
async fn alle_faner_viser_innlest_data() {
    let (_app, router) = app_med_data().await;
    let topic = topic_navn(&Topic::Hendelselogg);

    let (status, html) = hent(&router, "/stats/toppfelt", true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains("Advarsler</span><span class=\"verdi\">1</span>"),
        "{html}"
    );
    assert!(
        html.contains("Signaturfeil</span><span class=\"verdi\">1</span>"),
        "{html}"
    );

    let (status, html) = hent(&router, "/stats/?fane=oversikt", true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<td>Melding har trace_id</td>"), "{html}");
    assert!(!html.contains("<!DOCTYPE html>"));

    for sti in [
        "/stats/?fane=funn",
        "/stats/?fane=funn&regel=3",
        "/stats/?fane=funn&regel=3&alvorlighet=advarsel",
    ] {
        let (status, html) = hent(&router, sti, true).await;
        assert_eq!(status, StatusCode::OK, "{sti}");
        assert!(html.contains(&format!("<td>{topic}</td>")), "{sti}: {html}");
    }
    let (_, html) = hent(&router, "/stats/?fane=funn&alvorlighet=kritisk", true).await;
    assert!(html.contains("Ingen funn med dette filteret."));

    let (status, html) = hent(&router, "/stats/?fane=signatur", true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Mangler signatur"), "{html}");

    let (status, html) = hent(&router, "/stats/?fane=trace", true).await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains(&format!("<td>{topic}</td><td class=\"tall\">1</td>")),
        "{html}"
    );
}

#[tokio::test]
async fn vanlig_navigasjon_gir_hele_siden() {
    let (_app, router) = app_med_data().await;
    let (status, html) = hent(&router, "/stats/?fane=trace", false).await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains(r#"aria-current="page">Trace</a>"#));
}
