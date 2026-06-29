mod common;

use std::collections::HashSet;
use std::num::NonZeroU16;

use chrono::{Duration, Timelike, Utc};
use common::{aktiv_periode, avsluttet_periode, les_rad, setup};
use interne_hendelser::vo::Opplysning;
use paw_test::hendelse_builder::{rfc3339, StartetBuilder};
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use utgang::dao::les_periode::{hent_utdaterte_perioder, oppdater_periode};
use utgang::dao::skriv_periode::{skriv_periode_melding, skriv_startet_hendelse};
use utgang::dao::tilstand::Tilstand;
use uuid::Uuid;

#[tokio::test]
async fn skriv_startet_hendelse_setter_bekreftet_false_og_opplysninger_i_tilstand() {
    let pool = setup().await;
    let startet = StartetBuilder {
        opplysninger: HashSet::from([Opplysning::ErOver18Aar, Opplysning::HarNorskAdresse]),
        ..Default::default()
    }
    .build();
    let id = startet.hendelse_id;

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.unwrap();
    assert!(!rad.bekreftet);
    assert!(rad.stoppet.is_none());
    assert!(!rad.trenger_kontroll);
    let initielle = rad.tilstand.unwrap()["initielle"].as_array().unwrap().len();
    assert_eq!(initielle, 2);
}

#[tokio::test]
async fn skriv_startet_hendelse_setter_korrekt_arbeidssoeker_id() {
    let pool = setup().await;
    let startet = StartetBuilder { arbeidssoeker_id: 42, ..Default::default() }.build();
    let id = startet.hendelse_id;

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let (arbeidssoeker_id,): (Option<i64>,) =
        sqlx::query_as("SELECT arbeidssoeker_id FROM perioder WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(arbeidssoeker_id, Some(42));
}

#[tokio::test]
async fn skriv_periode_melding_aktiv_periode_har_ikke_stoppet() {
    let pool = setup().await;
    let id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    skriv_periode_melding(&mut tx, aktiv_periode(id)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.unwrap();
    assert!(rad.stoppet.is_none());
    assert!(rad.bekreftet);
}

#[tokio::test]
async fn skriv_periode_melding_avsluttet_periode_har_stoppet_satt() {
    let pool = setup().await;
    let id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    skriv_periode_melding(&mut tx, avsluttet_periode(id)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.unwrap();
    assert!(rad.stoppet.is_some());
    assert!(rad.stoppet.unwrap()["tidspunkt"].is_string());
}

/// Upsert fra periode-topic (tilstand=NULL) skal ikke overskrive tilstand satt av startet hendelse.
#[tokio::test]
async fn skriv_periode_melding_upsert_beholder_tilstand_fra_startet_hendelse() {
    let pool = setup().await;
    let id = Uuid::new_v4();

    let startet = StartetBuilder {
        hendelse_id: id,
        opplysninger: HashSet::from([Opplysning::ErOver18Aar, Opplysning::HarNorskAdresse]),
        ..Default::default()
    }
    .build();
    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    skriv_periode_melding(&mut tx, aktiv_periode(id)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.unwrap();
    let initielle = rad.tilstand.unwrap()["initielle"].as_array().unwrap().len();
    assert_eq!(initielle, 2, "COALESCE skal bevare tilstand fra startet hendelse");
    assert!(rad.bekreftet, "bekreftet: false OR true = true");
}

#[tokio::test]
async fn hent_utdaterte_perioder_returnerer_kun_eldre_enn_vannmerke() {
    let pool = setup().await;
    let vannmerke = Utc::now();

    let gammel_id = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(
        &mut tx,
        StartetBuilder {
            hendelse_id: gammel_id,
            identitetsnummer: "10000000001".to_string(),
            utfoert_av_id: "10000000001".to_string(),
            tidspunkt: rfc3339("2024-01-01T00:00:00Z"),
            ..Default::default()
        }
        .build(),
    )
    .await
    .unwrap();
    skriv_startet_hendelse(
        &mut tx,
        StartetBuilder {
            identitetsnummer: "20000000002".to_string(),
            utfoert_av_id: "20000000002".to_string(),
            tidspunkt: Utc::now() + Duration::hours(1),
            ..Default::default()
        }
        .build(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_utdaterte_perioder(&mut tx, vannmerke, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert_eq!(rader[0].id, ArbeidssoekerperiodeId::from(gammel_id));
}

#[tokio::test]
async fn hent_utdaterte_perioder_ekskluderer_trenger_kontroll_true() {
    let pool = setup().await;
    let vannmerke = Utc::now() + Duration::hours(1);
    let gammelt_tidspunkt = rfc3339("2024-01-01T00:00:00Z");

    let uten_id = Uuid::new_v4();
    let med_id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    for (id, ident) in [(uten_id, "10000000001"), (med_id, "20000000002")] {
        skriv_startet_hendelse(
            &mut tx,
            StartetBuilder {
                hendelse_id: id,
                identitetsnummer: ident.to_string(),
                utfoert_av_id: ident.to_string(),
                tidspunkt: gammelt_tidspunkt,
                ..Default::default()
            }
            .build(),
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    oppdater_periode(&mut tx, ArbeidssoekerperiodeId::from(med_id), gammelt_tidspunkt, true, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_utdaterte_perioder(&mut tx, vannmerke, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert_eq!(rader[0].id, ArbeidssoekerperiodeId::from(uten_id));
}

#[tokio::test]
async fn hent_utdaterte_perioder_respekterer_limit() {
    let pool = setup().await;
    let vannmerke = Utc::now() + Duration::hours(1);
    let gammelt_tidspunkt = rfc3339("2024-01-01T00:00:00Z");

    let mut tx = pool.begin().await.unwrap();
    for i in 0..5u8 {
        let ident = format!("1000000000{i}");
        skriv_startet_hendelse(
            &mut tx,
            StartetBuilder {
                hendelse_id: Uuid::new_v4(),
                identitetsnummer: ident.clone(),
                utfoert_av_id: ident,
                tidspunkt: gammelt_tidspunkt,
                ..Default::default()
            }
            .build(),
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_utdaterte_perioder(&mut tx, vannmerke, NonZeroU16::new(3).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 3);
}

#[tokio::test]
async fn oppdater_periode_endrer_sist_oppdatert_og_trenger_kontroll() {
    let pool = setup().await;
    let startet = StartetBuilder::default().build();
    let id = ArbeidssoekerperiodeId::from(startet.hendelse_id);

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let ny_tid = (Utc::now() + Duration::hours(1)).with_nanosecond(0).unwrap();
    let mut tx = pool.begin().await.unwrap();
    oppdater_periode(&mut tx, id.clone(), ny_tid, true, None).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id.0).await.unwrap();
    assert!(rad.trenger_kontroll);
    assert_eq!(rad.sist_oppdatert, ny_tid.naive_utc());
}

#[tokio::test]
async fn oppdater_periode_kan_sette_ny_tilstand() {
    let pool = setup().await;
    let startet = StartetBuilder {
        opplysninger: HashSet::from([Opplysning::ErOver18Aar]),
        ..Default::default()
    }
    .build();
    let id = ArbeidssoekerperiodeId::from(startet.hendelse_id);

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let ny_tilstand = Tilstand { initielle: vec![Opplysning::ErOver18Aar], gjeldende: None, forrige: None };
    let mut tx = pool.begin().await.unwrap();
    oppdater_periode(&mut tx, id.clone(), Utc::now(), false, Some(ny_tilstand)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id.0).await.unwrap();
    assert!(rad.tilstand.is_some());
}
