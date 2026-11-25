mod common;

use std::time::Duration;

use arbeidssoekerregisteret_internkontroll::features::regler::{
    funn::hent_modne_funn, startet_gir_periode::StartetGirPeriode,
};
use common::{TestApp, melding::Melding};
use paw_test::hendelse_builder::StartetBuilder;
use uuid::Uuid;

const T: i64 = 1_704_110_400_000;
const VINDU: Duration = Duration::from_secs(60);

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
    hent_modne_funn(&mut conn, VINDU).await.unwrap().len()
}

#[tokio::test]
async fn funn_blir_modent_naar_stream_time_passerer_vinduet() {
    let app = TestApp::start().await;
    app.les_inn(startet(T)).await;
    app.les_inn(startet(T + 59_999)).await;
    app.kjor_regler(vec![Box::new(StartetGirPeriode)]).await;
    assert_eq!(app.funn().await.len(), 2);
    assert_eq!(modne(&app).await, 0);

    app.les_inn(startet(T + 60_000)).await;
    app.kjor_regler(vec![Box::new(StartetGirPeriode)]).await;

    assert_eq!(app.funn().await.len(), 3);
    assert_eq!(modne(&app).await, 1);
}
