//! Visualiseringen mot ekte database: alle faner og filtre svarer, og
//! innholdet stemmer med det som er lest inn.

mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    kjoring::kjor_runde, regel::registrer_regler, startet_er_signert::StartetErSignert,
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

async fn hent(app: &Router, sti: &str) -> (StatusCode, String) {
    let req = Request::get(sti);
    let response = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, String::from_utf8(body.to_vec()).unwrap())
}

/// Leser inn én startet-melding uten trace og signatur, og kjører regelen.
const T: i64 = 1_704_110_400_000;

async fn app_med_data() -> (TestApp, Router) {
    let app = TestApp::start().await;
    app.les_inn(Melding::ny(StartetBuilder::default()).tidspunkt_ms(T))
        .await;
    app.les_inn(
        Melding::ny(StartetBuilder::default())
            .tidspunkt_ms(T + 60_000)
            .header(
                "traceparent",
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            ),
    )
    .await;
    sqlx::query(
        "UPDATE record_metadata SET signature_status = 'missing_signature' WHERE record_offset = 0",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let regler = registrer_regler(
        vec![Box::new(StartetErSignert)],
        &app.topic_ids,
        &RuntimeEnv::Local,
    );
    let tildelt = AssignedPartitions::default();
    tildelt.add([TopicPartition {
        topic: topic_navn(&Topic::Hendelselogg).to_string(),
        partition: 0,
    }]);
    while kjor_runde(&app.pool, &regler, &tildelt, 10).await.unwrap() > 0 {}
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

    let (status, html) = hent(&router, "/stats/fragment/toppfelt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains("Kritiske funn</span><span class=\"verdi\">1</span>"),
        "{html}"
    );
    assert!(
        html.contains("Signaturfeil</span><span class=\"verdi\">1</span>"),
        "{html}"
    );

    let (status, html) = hent(&router, "/stats/fragment/fane?fane=oversikt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("<summary>Startet-hendelsen er signert</summary>"), "{html}");
    // Uten HWM vet vi ikke om regelen er ajour, så eldste stream-tid vises.
    assert!(
        html.contains("<td>2024-01-01 12:01:00.000 UTC</td>"),
        "{html}"
    );
    assert!(!html.contains("<!DOCTYPE html>"));

    for sti in [
        "/stats/fragment/fane?fane=funn",
        "/stats/fragment/fane?fane=funn&regel=3",
        "/stats/fragment/fane?fane=funn&regel=3&alvorlighet=kritisk",
    ] {
        let (status, html) = hent(&router, sti).await;
        assert_eq!(status, StatusCode::OK, "{sti}");
        assert!(html.contains(&format!("<td>{topic}</td>")), "{sti}: {html}");
    }
    let (_, html) = hent(
        &router,
        "/stats/fragment/fane?fane=funn&alvorlighet=advarsel",
    )
    .await;
    assert!(html.contains("Ingen funn med dette filteret."));

    let (status, html) = hent(&router, "/stats/fragment/fane?fane=signatur").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("Mangler signatur"), "{html}");

    let (status, html) = hent(&router, "/stats/fragment/fane?fane=trace").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        html.contains(&format!("<td>{topic}</td><td class=\"tall\">1</td>")),
        "{html}"
    );
}

/// En regel som har nådd HWM er ajour, selv om stream-tiden i partisjonen er
/// gammel fordi det ikke har kommet nye meldinger.
#[tokio::test]
async fn regel_som_har_naadd_hwm_er_ajour() {
    let (app, router) = app_med_data().await;
    sqlx::query("INSERT INTO hwm (version, topic, partition, hwm) VALUES (1, $1, 0, 1)")
        .bind(topic_navn(&Topic::Hendelselogg))
        .execute(&app.pool)
        .await
        .unwrap();
    let (status, html) = hent(&router, "/stats/fragment/fane?fane=oversikt").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(r#"aria-label="100 % ajour""#), "{html}");
}

#[tokio::test]
async fn hele_siden_virker_uten_javascript() {
    let (_app, router) = app_med_data().await;
    let (status, html) = hent(&router, "/stats/?fane=trace").await;
    assert_eq!(status, StatusCode::OK);
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains(r#"aria-current="page">Trace</a>"#));
    // Toppfelt og fane er rendret på serveren, så siden virker uten JavaScript.
    assert!(html.contains("Signaturfeil</span><span class=\"verdi\">1</span>"));
    assert!(html.contains("<h2>Meldinger uten trace_id</h2>"));
    // Eneste skript er vårt eget, og ingen inline-skript.
    assert_eq!(html.matches("<script").count(), 1);
    assert!(html.contains(r#"<script src="/stats/static/oppdater.js" defer></script>"#));
}
