use std::collections::HashSet;
use std::num::NonZeroU16;

use interne_hendelser::vo::Opplysning;
use paw_test::setup_test_db::setup_test_db;
use utgang::{
    db_read_ops::hent_klar_for_kontroll,
    db_write_ops::{skriv_pdl_info_batch, skrive_startet_hendelse},
};

mod common;

use crate::common::hendelse_startet;

async fn setup(pool: &sqlx::PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");
}

async fn insert_periode_metadata(pool: &sqlx::PgPool, periode_id: uuid::Uuid) {
    sqlx::query(
        "INSERT INTO periode_metadata (periode_id, identitetsnummer, arbeidssoeker_id, kafka_key) VALUES ($1, $2, $3, $4)",
    )
    .bind(periode_id)
    .bind("12345678901")
    .bind(1_i64)
    .bind(1_i64)
    .execute(pool)
    .await
    .expect("Failed to insert periode_metadata");
}

#[tokio::test]
async fn ingen_rader_returnerer_tom_liste() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    setup(&pool).await;

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_klar_for_kontroll(&mut tx, &NonZeroU16::new(10).unwrap())
        .await
        .expect("Failed to fetch klar_for_kontroll");
    tx.commit().await.unwrap();

    assert!(rader.is_empty(), "Expected empty result when no rows exist");
}

#[tokio::test]
async fn returnerer_rad_med_startet_opplysninger_uten_forrige_pdl() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    setup(&pool).await;

    let startet = hendelse_startet();
    let periode_id = startet.hendelse_id;
    let forventede_startet: HashSet<Opplysning> = startet.opplysninger.clone();

    let mut tx = pool.begin().await.unwrap();
    skrive_startet_hendelse(&mut tx, &startet, 1)
        .await
        .expect("Failed to write startet hendelse");
    tx.commit().await.unwrap();

    let pdl_opplysninger = vec![Opplysning::ErOver18Aar, Opplysning::HarNorskAdresse];
    let mut tx = pool.begin().await.unwrap();
    skriv_pdl_info_batch(&mut tx, vec![(periode_id, pdl_opplysninger.clone())])
        .await
        .expect("Failed to write pdl info");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_klar_for_kontroll(&mut tx, &NonZeroU16::new(10).unwrap())
        .await
        .expect("Failed to fetch klar_for_kontroll");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    let rad = rader.first().unwrap();
    assert_eq!(rad.periode_id, periode_id);

    let lest_pdl: HashSet<Opplysning> = rad.opplysninger.iter().cloned().collect();
    assert_eq!(lest_pdl, pdl_opplysninger.into_iter().collect());

    let lest_startet: HashSet<Opplysning> = rad
        .startet_opplysninger
        .as_ref()
        .expect("startet_opplysninger should be Some")
        .iter()
        .cloned()
        .collect();
    assert_eq!(lest_startet, forventede_startet);

    assert!(
        rad.forrige_pdl_opplysninger.is_none(),
        "forrige_pdl_opplysninger should be None for the first PDL check"
    );
}

#[tokio::test]
async fn returnerer_forrige_pdl_opplysninger_naar_to_pdl_sjekker() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    setup(&pool).await;

    let startet = hendelse_startet();
    let periode_id = startet.hendelse_id;

    let mut tx = pool.begin().await.unwrap();
    skrive_startet_hendelse(&mut tx, &startet, 1)
        .await
        .expect("Failed to write startet hendelse");
    tx.commit().await.unwrap();

    let foerste_pdl = vec![Opplysning::ErOver18Aar];
    let andre_pdl = vec![Opplysning::ErOver18Aar, Opplysning::HarNorskAdresse];

    let mut tx = pool.begin().await.unwrap();
    skriv_pdl_info_batch(&mut tx, vec![(periode_id, foerste_pdl.clone())])
        .await
        .expect("Failed to write first pdl info");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    skriv_pdl_info_batch(&mut tx, vec![(periode_id, andre_pdl.clone())])
        .await
        .expect("Failed to write second pdl info");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_klar_for_kontroll(&mut tx, &NonZeroU16::new(10).unwrap())
        .await
        .expect("Failed to fetch klar_for_kontroll");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 2, "Expected 2 rows (one per PdlSjekk)");

    let foerste_rad = &rader[0];
    assert_eq!(foerste_rad.periode_id, periode_id);
    assert!(
        foerste_rad.forrige_pdl_opplysninger.is_none(),
        "First PdlSjekk should have no previous PDL opplysninger"
    );

    let andre_rad = &rader[1];
    assert_eq!(andre_rad.periode_id, periode_id);
    let lest_forrige: HashSet<Opplysning> = andre_rad
        .forrige_pdl_opplysninger
        .as_ref()
        .expect("Second PdlSjekk should have forrige_pdl_opplysninger")
        .iter()
        .cloned()
        .collect();
    assert_eq!(
        lest_forrige,
        foerste_pdl.into_iter().collect::<HashSet<_>>()
    );
}

#[tokio::test]
async fn startet_opplysninger_er_none_naar_ingen_startet_hendelse() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    setup(&pool).await;

    let periode_id = uuid::Uuid::new_v4();
    insert_periode_metadata(&pool, periode_id).await;

    let pdl_opplysninger = vec![Opplysning::ErOver18Aar];
    let mut tx = pool.begin().await.unwrap();
    skriv_pdl_info_batch(&mut tx, vec![(periode_id, pdl_opplysninger)])
        .await
        .expect("Failed to write pdl info");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_klar_for_kontroll(&mut tx, &NonZeroU16::new(10).unwrap())
        .await
        .expect("Failed to fetch klar_for_kontroll");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert!(
        rader[0].startet_opplysninger.is_none(),
        "startet_opplysninger should be None when no StartetHendelse row exists"
    );
}

#[tokio::test]
async fn limit_begrenser_antall_rader() {
    let (pool, _container) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    setup(&pool).await;

    for kafka_key in 1_i64..=3 {
        let startet = hendelse_startet();
        let mut tx = pool.begin().await.unwrap();
        skrive_startet_hendelse(&mut tx, &startet, kafka_key)
            .await
            .expect("Failed to write startet hendelse");
        skriv_pdl_info_batch(
            &mut tx,
            vec![(startet.hendelse_id, vec![Opplysning::ErOver18Aar])],
        )
        .await
        .expect("Failed to write pdl info");
        tx.commit().await.unwrap();
    }

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_klar_for_kontroll(&mut tx, &NonZeroU16::new(2).unwrap())
        .await
        .expect("Failed to fetch klar_for_kontroll");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 2, "Expected limit to cap results at 2");
}
