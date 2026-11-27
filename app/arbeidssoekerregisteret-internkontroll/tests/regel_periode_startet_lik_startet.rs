mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    periode_startet_lik_startet::PeriodeStartetLikStartet, regel::Regel,
};
use common::{TestApp, melding::Melding};
use eksterne_hendelser::periode::Periode;
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::{StartetBuilder, rfc3339};
use test_data_generator::eksterne_hendelser::create_dummy_start_periode;
use uuid::Uuid;

const T: i64 = 1_704_110_400_000;
const FNR: &str = "12345678901";
const TIDSPUNKT: &str = "2024-01-01T12:00:00.123Z";

fn startet(periode_id: Uuid) -> StartetBuilder {
    StartetBuilder {
        hendelse_id: periode_id,
        identitetsnummer: FNR.into(),
        utfoert_av_id: FNR.into(),
        tidspunkt: rfc3339(TIDSPUNKT),
        ..Default::default()
    }
}

fn periode(periode_id: Uuid) -> Periode {
    create_dummy_start_periode(FNR, periode_id, Some(rfc3339(TIDSPUNKT)))
}

fn regel() -> Vec<Box<dyn Regel>> {
    vec![Box::new(PeriodeStartetLikStartet)]
}

async fn kjor(app: &TestApp, startet: StartetBuilder, periode: Periode, startet_foerst: bool) {
    if startet_foerst {
        app.les_inn(Melding::ny(startet).tidspunkt_ms(T)).await;
        app.les_inn(Melding::ny(periode).tidspunkt_ms(T + 10)).await;
    } else {
        app.les_inn(Melding::ny(periode).tidspunkt_ms(T)).await;
        app.les_inn(Melding::ny(startet).tidspunkt_ms(T + 10)).await;
    }
    app.kjor_regler(regel()).await;
}

async fn koder(app: &TestApp) -> Vec<i16> {
    app.funn()
        .await
        .into_iter()
        .map(|(_, _, kode)| kode)
        .collect()
}

#[tokio::test]
async fn like_gir_ikke_funn_uansett_rekkefoelge() {
    for startet_foerst in [true, false] {
        let app = TestApp::start().await;
        let id = Uuid::new_v4();
        kjor(&app, startet(id), periode(id), startet_foerst).await;
        assert!(
            app.funn().await.is_empty(),
            "startet først: {startet_foerst}"
        );
    }
}

#[tokio::test]
async fn avvik_gir_funn_med_kode_for_feltet_uansett_rekkefoelge() {
    for startet_foerst in [true, false] {
        let id = Uuid::new_v4();
        let tilfeller: [(StartetBuilder, Periode, i16); 3] = [
            (
                StartetBuilder {
                    identitetsnummer: "99999999999".into(),
                    ..startet(id)
                },
                periode(id),
                1,
            ),
            (
                StartetBuilder {
                    utfoert_av_id: "Z123456".into(),
                    ..startet(id)
                },
                periode(id),
                2,
            ),
            (
                startet(id),
                create_dummy_start_periode(FNR, id, Some(rfc3339("2024-01-01T12:00:01.124Z"))),
                3,
            ),
        ];
        for (startet, periode, kode) in tilfeller {
            let app = TestApp::start().await;
            kjor(&app, startet, periode, startet_foerst).await;
            assert_eq!(koder(&app).await, [kode], "startet først: {startet_foerst}");
        }
    }
}

#[tokio::test]
async fn periode_uten_startet_er_ikke_relevant() {
    let app = TestApp::start().await;
    app.les_inn(Melding::ny(periode(Uuid::new_v4())).tidspunkt_ms(T))
        .await;
    app.kjor_regler(regel()).await;
    assert!(app.funn().await.is_empty());
}

#[tokio::test]
async fn tidsavvik_innenfor_feilmarginen_gir_ikke_funn() {
    for startet_foerst in [true, false] {
        for tidspunkt in [
            "2024-01-01T12:00:00.130Z",
            "2024-01-01T12:00:01.123Z",
            "2024-01-01T11:59:59.123Z",
        ] {
            let app = TestApp::start().await;
            let id = Uuid::new_v4();
            let periode = create_dummy_start_periode(FNR, id, Some(rfc3339(tidspunkt)));
            kjor(&app, startet(id), periode, startet_foerst).await;
            assert!(
                app.funn().await.is_empty(),
                "startet først: {startet_foerst}, tidspunkt: {tidspunkt}"
            );
        }
    }
}

#[tokio::test]
async fn funn_peker_paa_begge_meldingene_og_har_tidsavvik() {
    let id = Uuid::new_v4();
    let app = TestApp::start().await;
    let periode = create_dummy_start_periode(FNR, id, Some(rfc3339("2024-01-01T12:00:05.333Z")));
    kjor(&app, startet(id), periode, true).await;
    let hendelselogg = common::topic_navn(&Topic::Hendelselogg).to_string();
    let periode_topic = common::topic_navn(&Topic::Periode).to_string();
    // Ett funn per periode. Perioden kom sist, så den bestemte funnet og står først.
    assert_eq!(
        app.funn_grunnlag().await,
        [(
            vec![(periode_topic, 0), (hendelselogg, 0)],
            serde_json::json!({ "tidsavvik_ms": 5210 })
        )]
    );
}
