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
    assert!(
        html.contains("<summary>Startet-hendelsen er signert</summary>"),
        "{html}"
    );
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

mod funndetaljer {
    use super::*;
    use arbeidssoekerregisteret_internkontroll::features::regler::periode_startet_lik_startet::PeriodeStartetLikStartet;
    use interne_hendelser::vo::BrukerType;
    use paw_test::hendelse_builder::rfc3339;
    use test_data_generator::eksterne_hendelser::{
        create_dummy_avslutt_periode, create_dummy_start_periode,
    };
    use uuid::Uuid;

    const FNR_STARTET: &str = "12345678901";
    const FNR_PERIODE: &str = "10987654321";

    /// Startet-hendelse fra en veileder og en periode for en annen person.
    async fn app_med_funn() -> (TestApp, Router, String) {
        let app = TestApp::start().await;
        let id = Uuid::new_v4();
        let tid = "2024-01-01T12:00:00.000Z";
        app.les_inn(
            Melding::ny(StartetBuilder {
                hendelse_id: id,
                identitetsnummer: FNR_STARTET.into(),
                bruker_type: BrukerType::Veileder,
                utfoert_av_id: "Z123456".into(),
                tidspunkt: rfc3339(tid),
                ..Default::default()
            })
            .tidspunkt_ms(T),
        )
        .await;
        app.les_inn(
            Melding::ny(create_dummy_start_periode(
                FNR_PERIODE,
                id,
                Some(rfc3339(tid)),
            ))
            .tidspunkt_ms(T + 10),
        )
        .await;
        app.kjor_regler(vec![Box::new(PeriodeStartetLikStartet)])
            .await;
        let (regel, versjon, melding): (i16, i32, i64) =
            sqlx::query_as("SELECT regel, regelversjon, record_metadata_id FROM funn")
                .fetch_one(&app.pool)
                .await
                .unwrap();
        let sti = format!("/stats/funn?regel={regel}&versjon={versjon}&melding={melding}");
        let router = routes(
            VisState {
                pg_pool: app.pool.clone(),
                hwm_version: 1,
            },
            None,
        );
        (app, router, sti)
    }

    #[tokio::test]
    async fn viser_meldingene_uten_foedselsnummer() {
        let (_app, router, sti) = app_med_funn().await;
        let (status, html) = hent(&router, &sti).await;
        assert_eq!(status, StatusCode::OK, "{html}");
        assert!(html.contains("Ulikt fødselsnummer"), "{html}");
        // Perioden bestemte funnet og står først, så personen der er A.
        assert!(html.contains("<td>Person A</td>"), "{html}");
        assert!(html.contains("<td>Person B</td>"), "{html}");
        assert!(html.contains("<td>Z123456</td>"), "{html}");
        assert!(html.contains("avviker"), "{html}");
        assert!(!html.contains(FNR_STARTET), "{html}");
        assert!(!html.contains(FNR_PERIODE), "{html}");
    }

    #[tokio::test]
    async fn funn_listen_lenker_til_detaljene() {
        let (app, router, sti) = app_med_funn().await;
        // Listen viser bare modne funn: regelen må ha kommet ett minutt forbi.
        app.les_inn(Melding::ny(StartetBuilder::default()).tidspunkt_ms(T + 120_000))
            .await;
        app.kjor_regler(vec![Box::new(PeriodeStartetLikStartet)])
            .await;
        let (_, html) = hent(&router, "/stats/fragment/fane?fane=funn").await;
        assert!(html.contains(&sti.replace('&', "&amp;")), "{html}");
    }

    #[tokio::test]
    async fn ukjent_funn_gir_404_og_ugyldig_noekkel_gir_400() {
        let (_app, router, _) = app_med_funn().await;
        for (sti, forventet) in [
            (
                "/stats/funn?regel=5&versjon=2&melding=999999",
                StatusCode::NOT_FOUND,
            ),
            (
                "/stats/funn?regel=99&versjon=2&melding=1",
                StatusCode::BAD_REQUEST,
            ),
            (
                "/stats/funn?regel=5&versjon=2&melding=abc",
                StatusCode::BAD_REQUEST,
            ),
            ("/stats/funn?regel=5", StatusCode::BAD_REQUEST),
        ] {
            assert_eq!(hent(&router, sti).await.0, forventet, "{sti}");
        }
    }

    #[tokio::test]
    async fn viser_meldinger_for_perioden_rundt_funnet() {
        let (app, router, sti) = app_med_funn().await;
        // Funnet gjelder periode-meldingen. Avslutningen kommer etter og er ikke brukt.
        let id: uuid::Uuid = sqlx::query_scalar(
            "SELECT periode_id FROM record_metadata WHERE meldingstype = 'periode_startet'",
        )
        .fetch_one(&app.pool)
        .await
        .unwrap();
        app.les_inn(
            Melding::ny(create_dummy_avslutt_periode(
                FNR_PERIODE,
                id,
                Some(rfc3339("2024-01-01T12:00:00.000Z")),
                Some(rfc3339("2024-02-01T12:00:00.000Z")),
            ))
            .tidspunkt_ms(T + 20),
        )
        .await;

        let (status, html) = hent(&router, &sti).await;
        assert_eq!(status, StatusCode::OK, "{html}");
        let (_, kontekst) = html
            .split_once("Meldinger for perioden")
            .expect("mangler utsnittet");
        assert_eq!(
            kontekst.matches("<tr").count(),
            4,
            "tittel og tre meldinger: {kontekst}"
        );
        assert_eq!(
            kontekst.matches("Funnet gjelder denne").count(),
            1,
            "{kontekst}"
        );
        assert_eq!(
            kontekst.matches("Brukt av regelen").count(),
            1,
            "{kontekst}"
        );
        assert!(kontekst.contains("periode_avsluttet"), "{kontekst}");
        // Samme bokstav som i listen over: perioden er A, startet-hendelsen B.
        // Periodemeldingene er utført av sluttbrukeren selv, så A står to ganger per rad.
        assert_eq!(
            kontekst.matches("<td>Person A</td>").count(),
            4,
            "{kontekst}"
        );
        assert_eq!(
            kontekst.matches("<td>Person B</td>").count(),
            1,
            "{kontekst}"
        );
        assert!(!html.contains(FNR_STARTET), "{html}");
        assert!(!html.contains(FNR_PERIODE), "{html}");
    }
}
