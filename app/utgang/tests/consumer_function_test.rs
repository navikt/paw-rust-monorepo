mod common;

use chrono::{Duration, Utc};
use paw_test::setup_test_db::setup_test_db;
use std::num::NonZeroU16;
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use utgang::dao::periode_rad::{
    hent_perioder, hent_perioder_eldre_enn, oppdater_sist_oppdatert, skriv_perioder,
};
use uuid::Uuid;

use common::{main_avro_metadata, main_avro_periode};

async fn setup() -> sqlx::PgPool {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

/// Verifiserer at en avsluttet periode fra periode-topicen markeres som stoppet
/// og dermed ikke plukkes opp av PDL-oppdateringen.
#[tokio::test]
async fn avsluttet_periode_fra_periode_topic_ekskluderes_fra_pdl_oppdatering() {
    let pool = setup().await;

    let mut periode = main_avro_periode();
    periode.avsluttet = Some(main_avro_metadata());

    let periode_rad: utgang::dao::periode_rad::PeriodeRad = (&periode).into();
    assert!(
        periode_rad.stoppet,
        "PeriodeRad fra avsluttet periode skal ha stoppet=true"
    );

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, &[periode_rad]).await.unwrap();
    tx.commit().await.unwrap();

    let grense = Utc::now() + Duration::hours(1);
    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert!(
        rader.is_empty(),
        "Avsluttet periode skal ikke returneres for PDL-oppdatering"
    );
}

/// Verifiserer at en aktiv periode fra periode-topicen plukkes opp av PDL-oppdatering.
#[tokio::test]
async fn aktiv_periode_fra_periode_topic_inkluderes_i_pdl_oppdatering() {
    let pool = setup().await;

    let periode = main_avro_periode();
    assert!(periode.avsluttet.is_none());

    let periode_rad: utgang::dao::periode_rad::PeriodeRad = (&periode).into();
    assert!(!periode_rad.stoppet);

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, &[periode_rad]).await.unwrap();
    tx.commit().await.unwrap();

    let grense = Utc::now() + Duration::hours(1);
    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1, "Aktiv periode skal inkluderes");
}

/// Verifiserer at upsert fra periode-topic med avsluttet korrekt oppdaterer
/// en eksisterende aktiv periode til stoppet.
#[tokio::test]
async fn upsert_med_avsluttet_oppdaterer_aktiv_periode_til_stoppet() {
    let pool = setup().await;

    let mut periode = main_avro_periode();
    let periode_id = periode.id;

    let aktiv_rad: utgang::dao::periode_rad::PeriodeRad = (&periode).into();
    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, &[aktiv_rad]).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[ArbeidssoekerperiodeId::from(periode_id)])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(rader.len(), 1);
    assert!(!rader[0].stoppet);

    periode.avsluttet = Some(main_avro_metadata());
    let stoppet_rad: utgang::dao::periode_rad::PeriodeRad = (&periode).into();
    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, &[stoppet_rad]).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[ArbeidssoekerperiodeId::from(periode_id)])
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(
        rader.is_empty(),
        "Stoppet periode skal ikke returneres av hent_perioder (filtrerer stoppet=false)"
    );
}

/// Verifiserer at perioder som sjekkes mot PDL uten endringer ikke plukkes
/// opp igjen dersom sist_oppdatert oppdateres etter sjekk.
#[tokio::test]
async fn periode_uten_endring_skal_ikke_plukkes_opp_igjen_etter_pdl_sjekk() {
    let pool = setup().await;

    let periode = main_avro_periode();
    let periode_id = ArbeidssoekerperiodeId::from(periode.id);

    let periode_rad: utgang::dao::periode_rad::PeriodeRad = (&periode).into();
    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, &[periode_rad]).await.unwrap();
    tx.commit().await.unwrap();

    // Perioden har sist_oppdatert = startet.tidspunkt (2024-01-01)
    // Vannmerke er nå + 1h, så perioden plukkes opp
    let naa = Utc::now();
    let grense = naa + Duration::hours(1);
    let mut tx = pool.begin().await.unwrap();
    let foerste_kjoring = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(foerste_kjoring.len(), 1, "Perioden skal plukkes opp");

    // Simuler at PDL-sjekk er gjort uten endringer — oppdater sist_oppdatert
    let mut tx = pool.begin().await.unwrap();
    oppdater_sist_oppdatert(&mut tx, &[periode_id], naa)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Med grense = naa - 1s skal perioden IKKE plukkes opp igjen
    // fordi sist_oppdatert (naa) er nyere enn grensen
    let eldre_grense = naa - Duration::seconds(1);
    let mut tx = pool.begin().await.unwrap();
    let andre_kjoring =
        hent_perioder_eldre_enn(&mut tx, eldre_grense, NonZeroU16::new(100).unwrap())
            .await
            .unwrap();
    tx.commit().await.unwrap();

    assert!(
        andre_kjoring.is_empty(),
        "Periode med oppdatert sist_oppdatert skal ikke plukkes opp igjen"
    );
}

/// Verifiserer at oppdater_sist_oppdatert faktisk forhindrer re-plukking
/// når grensen er lik eller eldre enn sist_oppdatert.
#[tokio::test]
async fn oppdatert_sist_oppdatert_forhindrer_re_plukking() {
    let pool = setup().await;

    let periode = main_avro_periode();
    let periode_id = ArbeidssoekerperiodeId::from(periode.id);

    let periode_rad: utgang::dao::periode_rad::PeriodeRad = (&periode).into();
    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, &[periode_rad]).await.unwrap();
    tx.commit().await.unwrap();

    let naa = Utc::now();
    let grense_vid = naa + Duration::hours(1);

    // Plukkes opp med vid grense
    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense_vid, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(rader.len(), 1);

    // Oppdater sist_oppdatert til nå
    let mut tx = pool.begin().await.unwrap();
    oppdater_sist_oppdatert(&mut tx, &[periode_id], naa)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    // Med grense eldre enn sist_oppdatert: perioden filtreres bort
    let eldre_grense = naa - Duration::seconds(1);
    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, eldre_grense, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert!(
        rader.is_empty(),
        "Periode med oppdatert sist_oppdatert skal ikke plukkes opp igjen"
    );
}
