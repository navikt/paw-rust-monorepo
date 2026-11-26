mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    funn::hent_modne_funn, regel::Regel, startet_gir_periode::StartetGirPeriode,
};
use common::{TestApp, melding::Melding};
use paw_test::hendelse_builder::StartetBuilder;
use uuid::Uuid;

const T: i64 = 1_704_110_400_000;

fn startet(tid: i64) -> Melding<StartetBuilder> {
    Melding::ny(StartetBuilder {
        hendelse_id: Uuid::new_v4(),
        arbeidssoeker_id: tid,
        ..Default::default()
    })
    .tidspunkt_ms(tid)
}

async fn modne(app: &TestApp) -> usize {
    let mut conn = app.pool.acquire().await.unwrap();
    hent_modne_funn(&mut conn).await.unwrap().len()
}

fn regel() -> Vec<Box<dyn Regel>> {
    vec![Box::new(StartetGirPeriode)]
}

#[tokio::test]
async fn stille_partisjon_modnes_av_stream_time_i_andre_partisjoner() {
    let app = TestApp::start().await;
    app.les_inn(startet(T).partisjon(1)).await;
    app.les_inn(startet(T + 60_000).partisjon(0)).await;

    app.kjor_regler_paa(regel(), &[0, 1]).await;

    assert_eq!(app.funn().await.len(), 2);
    assert_eq!(modne(&app).await, 1);
}

#[tokio::test]
async fn partisjon_med_uleste_meldinger_venter_paa_egen_stream_time() {
    let app = TestApp::start().await;
    app.les_inn(startet(T).partisjon(1)).await;
    app.les_inn(startet(T + 60_000).partisjon(0)).await;
    app.kjor_regler_paa(regel(), &[0, 1]).await;
    app.les_inn(startet(T + 10).partisjon(1)).await;

    assert_eq!(modne(&app).await, 0);
}

#[tokio::test]
async fn funn_blir_modent_naar_stream_time_passerer_vinduet() {
    let app = TestApp::start().await;
    app.les_inn(startet(T)).await;
    app.les_inn(startet(T + 59_999)).await;
    app.kjor_regler(regel()).await;
    assert_eq!(app.funn().await.len(), 2);
    assert_eq!(modne(&app).await, 0);

    app.les_inn(startet(T + 60_000)).await;
    app.kjor_regler(regel()).await;

    assert_eq!(app.funn().await.len(), 3);
    assert_eq!(modne(&app).await, 1);
}
