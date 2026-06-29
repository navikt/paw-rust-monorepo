use std::collections::HashSet;
use std::num::NonZeroU16;

use chrono::{Duration, Timelike, Utc};
use interne_hendelser::vo::Opplysning;
use paw_test::hendelse_builder::{rfc3339, StartetBuilder};
use paw_test::setup_test_db::setup_test_db;
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use utgang::dao::les_periode::{hent_utdaterte_perioder, oppdater_periode};
use utgang::dao::skriv_periode::{skriv_periode_melding, skriv_startet_hendelse};
use utgang::dao::tilstand::Tilstand;
use utgang::kafka::periode_deserializer::{Bruker, BrukerType, Metadata, Periode};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct TestPeriodeRow {
    sist_oppdatert: chrono::NaiveDateTime,
    trenger_kontroll: bool,
    stoppet: Option<serde_json::Value>,
    tilstand: Option<serde_json::Value>,
    bekreftet: bool,
}

async fn setup() -> sqlx::PgPool {
    let (pool, _guard) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

async fn les_rad(pool: &sqlx::PgPool, id: Uuid) -> Option<TestPeriodeRow> {
    sqlx::query_as(
        "SELECT sist_oppdatert, trenger_kontroll, stoppet, tilstand, bekreftet \
         FROM perioder WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .unwrap()
}

fn avro_bruker() -> Bruker {
    Bruker {
        bruker_type: BrukerType::Sluttbruker,
        id: "12345678901".to_string(),
        sikkerhetsnivaa: None,
    }
}

fn avro_metadata(tidspunkt: chrono::DateTime<Utc>) -> Metadata {
    Metadata {
        tidspunkt,
        utfoert_av: avro_bruker(),
        kilde: "test".to_string(),
        aarsak: "test".to_string(),
        tidspunkt_fra_kilde: None,
    }
}

fn aktiv_periode(id: Uuid) -> Periode {
    Periode {
        id,
        identitetsnummer: "12345678901".to_string(),
        startet: avro_metadata(rfc3339("2024-01-01T00:00:00Z")),
        avsluttet: None,
    }
}

fn avsluttet_periode(id: Uuid) -> Periode {
    Periode {
        id,
        identitetsnummer: "12345678901".to_string(),
        startet: avro_metadata(rfc3339("2024-01-01T00:00:00Z")),
        avsluttet: Some(avro_metadata(rfc3339("2024-06-01T00:00:00Z"))),
    }
}

// --- skriv_startet_hendelse ---

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

    let rad = les_rad(&pool, id).await.expect("Perioden skal finnes");
    assert!(!rad.bekreftet, "Startet hendelse setter bekreftet=false");
    assert!(rad.stoppet.is_none(), "Startet hendelse setter ikke stoppet");
    assert!(!rad.trenger_kontroll, "Startet hendelse setter trenger_kontroll=false");

    let tilstand = rad.tilstand.expect("Tilstand skal være satt");
    let initielle = tilstand["initielle"].as_array().expect("initielle skal være array");
    assert_eq!(initielle.len(), 2, "Begge opplysninger lagres i tilstand.initielle");
}

#[tokio::test]
async fn skriv_startet_hendelse_setter_korrekt_arbeidssoeker_id_og_ident() {
    let pool = setup().await;
    let startet = StartetBuilder {
        arbeidssoeker_id: 42,
        identitetsnummer: "11223344556".to_string(),
        utfoert_av_id: "11223344556".to_string(),
        ..Default::default()
    }
    .build();
    let id = startet.hendelse_id;

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let rad: (Option<i64>, String) = sqlx::query_as(
        "SELECT arbeidssoeker_id, identitetsnummer FROM perioder WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rad.0, Some(42));
    assert_eq!(rad.1, "11223344556");
}

// --- skriv_periode_melding ---

#[tokio::test]
async fn skriv_periode_melding_aktiv_periode_har_ikke_stoppet() {
    let pool = setup().await;
    let id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    skriv_periode_melding(&mut tx, aktiv_periode(id)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.expect("Perioden skal finnes");
    assert!(rad.stoppet.is_none(), "Aktiv periode skal ikke ha stoppet");
    assert!(rad.bekreftet, "Periode-topic setter bekreftet=true");
}

#[tokio::test]
async fn skriv_periode_melding_avsluttet_periode_har_stoppet_satt() {
    let pool = setup().await;
    let id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    skriv_periode_melding(&mut tx, avsluttet_periode(id)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.expect("Perioden skal finnes");
    assert!(rad.stoppet.is_some(), "Avsluttet periode skal ha stoppet satt");
    let stoppet = rad.stoppet.unwrap();
    assert!(stoppet["tidspunkt"].is_string(), "stoppet.tidspunkt skal være satt");
}

#[tokio::test]
async fn skriv_periode_melding_upsert_beholder_tilstand_fra_startet_hendelse() {
    let pool = setup().await;
    let id = Uuid::new_v4();

    // Skriv startet hendelse — setter tilstand med opplysninger
    let startet = StartetBuilder {
        hendelse_id: id,
        opplysninger: HashSet::from([Opplysning::ErOver18Aar, Opplysning::HarNorskAdresse]),
        ..Default::default()
    }
    .build();
    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    // Skriv periode-melding med samme UUID — tilstand=NULL i upsert, COALESCE beholder eksisterende
    let mut tx = pool.begin().await.unwrap();
    skriv_periode_melding(&mut tx, aktiv_periode(id)).await.unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id).await.expect("Perioden skal finnes");
    let tilstand = rad.tilstand.expect("Tilstand skal fremdeles være satt");
    let initielle = tilstand["initielle"].as_array().expect("initielle skal være array");
    assert_eq!(initielle.len(), 2, "Tilstand fra startet hendelse skal bevares");
    assert!(
        rad.bekreftet,
        "bekreftet blir true (false OR true) etter periode-melding"
    );
}

// --- hent_utdaterte_perioder ---

#[tokio::test]
async fn hent_utdaterte_perioder_returnerer_kun_eldre_enn_vannmerke() {
    let pool = setup().await;
    let vannmerke = Utc::now();

    // Gammel periode: sist_oppdatert = 2024-01-01 (godt eldre enn nå)
    let gammel_id = Uuid::new_v4();
    let gammel = StartetBuilder {
        hendelse_id: gammel_id,
        identitetsnummer: "10000000001".to_string(),
        utfoert_av_id: "10000000001".to_string(),
        tidspunkt: rfc3339("2024-01-01T00:00:00Z"),
        ..Default::default()
    }
    .build();

    // Ny periode: sist_oppdatert = nå + 1 time (nyere enn vannmerke)
    let ny_id = Uuid::new_v4();
    let ny = StartetBuilder {
        hendelse_id: ny_id,
        identitetsnummer: "20000000002".to_string(),
        utfoert_av_id: "20000000002".to_string(),
        tidspunkt: Utc::now() + Duration::hours(1),
        ..Default::default()
    }
    .build();

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, gammel).await.unwrap();
    skriv_startet_hendelse(&mut tx, ny).await.unwrap();
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

    let uten_kontroll_id = Uuid::new_v4();
    let med_kontroll_id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(
        &mut tx,
        StartetBuilder {
            hendelse_id: uten_kontroll_id,
            identitetsnummer: "10000000001".to_string(),
            utfoert_av_id: "10000000001".to_string(),
            tidspunkt: gammelt_tidspunkt,
            ..Default::default()
        }
        .build(),
    )
    .await
    .unwrap();
    skriv_startet_hendelse(
        &mut tx,
        StartetBuilder {
            hendelse_id: med_kontroll_id,
            identitetsnummer: "20000000002".to_string(),
            utfoert_av_id: "20000000002".to_string(),
            tidspunkt: gammelt_tidspunkt,
            ..Default::default()
        }
        .build(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    // Sett trenger_kontroll=true på én av dem
    let mut tx = pool.begin().await.unwrap();
    oppdater_periode(
        &mut tx,
        ArbeidssoekerperiodeId::from(med_kontroll_id),
        gammelt_tidspunkt,
        true,
        None,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_utdaterte_perioder(&mut tx, vannmerke, NonZeroU16::new(100).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1, "Kun perioden uten trenger_kontroll skal returneres");
    assert_eq!(rader[0].id, ArbeidssoekerperiodeId::from(uten_kontroll_id));
}

#[tokio::test]
async fn hent_utdaterte_perioder_respekterer_limit() {
    let pool = setup().await;
    let vannmerke = Utc::now() + Duration::hours(1);
    let gammelt_tidspunkt = rfc3339("2024-01-01T00:00:00Z");

    let mut tx = pool.begin().await.unwrap();
    for i in 0..5u8 {
        let id_str = format!("1000000000{}", i);
        skriv_startet_hendelse(
            &mut tx,
            StartetBuilder {
                hendelse_id: Uuid::new_v4(),
                identitetsnummer: id_str.clone(),
                utfoert_av_id: id_str,
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

    assert_eq!(rader.len(), 3, "Limit på 3 skal respekteres");
}

// --- oppdater_periode ---

#[tokio::test]
async fn oppdater_periode_endrer_sist_oppdatert_og_trenger_kontroll() {
    let pool = setup().await;

    let startet = StartetBuilder::default().build();
    let id = ArbeidssoekerperiodeId::from(startet.hendelse_id);

    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();

    let ny_tid = Utc::now()
        .with_nanosecond(0)
        .unwrap()
        + Duration::hours(1);
    let mut tx = pool.begin().await.unwrap();
    oppdater_periode(&mut tx, id.clone(), ny_tid, true, None)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id.0).await.expect("Perioden skal finnes");
    assert!(rad.trenger_kontroll, "trenger_kontroll skal være true");
    assert_eq!(
        rad.sist_oppdatert,
        ny_tid.naive_utc(),
        "sist_oppdatert skal være oppdatert"
    );
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

    let ny_tilstand = Tilstand {
        initielle: vec![Opplysning::ErOver18Aar],
        gjeldende: None,
        forrige: None,
    };
    let mut tx = pool.begin().await.unwrap();
    oppdater_periode(&mut tx, id.clone(), Utc::now(), false, Some(ny_tilstand))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let rad = les_rad(&pool, id.0).await.expect("Perioden skal finnes");
    let tilstand = rad.tilstand.expect("Tilstand skal finnes");
    assert!(tilstand["initielle"].is_array(), "tilstand.initielle skal være array");
}
