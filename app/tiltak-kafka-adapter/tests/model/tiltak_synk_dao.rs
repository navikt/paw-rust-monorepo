use crate::support::test_data::{
    TestData, finn_tiltak_synk_batch, finn_tiltak_synk_by_periode_id, for_timer_siden, init_pool,
    oppdater_resultat, oppdater_status, opprett_arbeidssoeker, opprett_komplett,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;
use tiltak_kafka_adapter::model::dao::tiltak_synk;
use tiltak_kafka_adapter::model::dao::tiltak_synk::{Betingelse, select_batch};
use tiltak_kafka_adapter::model::status::SynkStatus;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn insert_manglende_oppretter_bare_manglende_og_er_idempotent() {
    let pool = init_pool().await;

    let ny_1 = TestData::from_identitetsnummer("01017012345");
    let ny_2 = TestData::from_identitetsnummer("02017012345");
    let mut eksisterende = TestData::from_identitetsnummer("03017012345");
    eksisterende.status = SynkStatus::Ferdig;
    eksisterende.antall_feil = 3;
    opprett_arbeidssoeker(&pool, &ny_1).await;
    opprett_arbeidssoeker(&pool, &ny_2).await;
    opprett_komplett(&pool, &eksisterende).await;

    let synk_id = Uuid::new_v4();

    let mut tx = pool.begin().await.unwrap();
    let insert_count_1 = tiltak_synk::insert_manglende(&mut tx, &synk_id)
        .await
        .unwrap();
    let insert_count_2 = tiltak_synk::insert_manglende(&mut tx, &synk_id)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(insert_count_1, 2);
    assert_eq!(insert_count_2, 0);

    for periode_id in [ny_1.periode_id, ny_2.periode_id] {
        let row = finn_tiltak_synk_by_periode_id(&pool, &periode_id).await;
        assert_eq!(row.status, SynkStatus::Venter);
        assert_eq!(row.synk_id, synk_id);
        assert_eq!(row.antall_feil, 0);
        assert_eq!(row.sist_kjoert, DateTime::<Utc>::UNIX_EPOCH);
    }
    let row = finn_tiltak_synk_by_periode_id(&pool, &eksisterende.periode_id).await;
    assert_eq!(row.status, SynkStatus::Ferdig);
    assert_eq!(row.synk_id, eksisterende.synk_id);
    assert_eq!(row.antall_feil, 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_antall_feil_grenser() {
    let pool = init_pool().await;

    let ni_feil = TestData::from_antall_feil(9);
    let ti_feil = TestData::from_antall_feil(10);

    opprett_komplett(&pool, &ni_feil).await;
    opprett_komplett(&pool, &ti_feil).await;

    let avbrutt_count = oppdater_status(
        &pool,
        SynkStatus::Feilet,
        SynkStatus::Avbrutt,
        Betingelse::AntallFeilMinst(10),
        false,
    )
    .await;
    let venter_count = oppdater_status(
        &pool,
        SynkStatus::Feilet,
        SynkStatus::Venter,
        Betingelse::AntallFeilUnder(10),
        false,
    )
    .await;

    let ni_feil_row = finn_tiltak_synk_by_periode_id(&pool, &ni_feil.periode_id).await;
    let ti_feil_row = finn_tiltak_synk_by_periode_id(&pool, &ti_feil.periode_id).await;

    assert_eq!(avbrutt_count, 1);
    assert_eq!(venter_count, 1);
    assert_eq!(ni_feil_row.status, SynkStatus::Venter);
    assert_eq!(ni_feil_row.antall_feil, 9);
    assert_eq!(ti_feil_row.status, SynkStatus::Avbrutt);
    assert_eq!(ti_feil_row.antall_feil, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_tidsgrenser_og_nullstilling() {
    let pool = init_pool().await;

    let mut aktiv_gammel = TestData::from_status(SynkStatus::Aktiv);
    aktiv_gammel.status_endret = for_timer_siden(3);
    let aktiv_ny = TestData::from_status(SynkStatus::Aktiv);
    let mut ferdig_gammel = TestData::from_status(SynkStatus::Ferdig);
    ferdig_gammel.sist_kjoert = for_timer_siden(3);
    ferdig_gammel.antall_feil = 4;
    let mut ferdig_ny = TestData::from_status(SynkStatus::Ferdig);
    ferdig_ny.antall_feil = 4;
    let mut avbrutt_gammel = TestData::from_status(SynkStatus::Avbrutt);
    avbrutt_gammel.status_endret = for_timer_siden(3);

    opprett_komplett(&pool, &aktiv_gammel).await;
    opprett_komplett(&pool, &aktiv_ny).await;
    opprett_komplett(&pool, &ferdig_gammel).await;
    opprett_komplett(&pool, &ferdig_ny).await;
    opprett_komplett(&pool, &avbrutt_gammel).await;

    let aktive_count = oppdater_status(
        &pool,
        SynkStatus::Aktiv,
        SynkStatus::Venter,
        Betingelse::StatusEndretFoer(for_timer_siden(3)),
        false,
    )
    .await;
    let ferdige_count = oppdater_status(
        &pool,
        SynkStatus::Ferdig,
        SynkStatus::Venter,
        Betingelse::SistKjoertFoer(for_timer_siden(3)),
        true,
    )
    .await;

    let aktiv_gammel_row = finn_tiltak_synk_by_periode_id(&pool, &aktiv_gammel.periode_id).await;
    let aktiv_ny_row = finn_tiltak_synk_by_periode_id(&pool, &aktiv_ny.periode_id).await;
    let ferdig_gammel_row = finn_tiltak_synk_by_periode_id(&pool, &ferdig_gammel.periode_id).await;
    let ferdig_ny_row = finn_tiltak_synk_by_periode_id(&pool, &ferdig_ny.periode_id).await;
    let avbrutt_gammel_row =
        finn_tiltak_synk_by_periode_id(&pool, &avbrutt_gammel.periode_id).await;

    assert_eq!(aktive_count, 1);
    assert_eq!(ferdige_count, 1);
    assert_eq!(aktiv_gammel_row.status, SynkStatus::Venter);
    assert_eq!(aktiv_gammel_row.antall_feil, 0);
    assert_eq!(aktiv_ny_row.status, SynkStatus::Aktiv);
    assert_eq!(aktiv_ny_row.antall_feil, 0);
    assert_eq!(ferdig_gammel_row.status, SynkStatus::Venter);
    assert_eq!(ferdig_gammel_row.antall_feil, 0);
    assert_eq!(ferdig_ny_row.status, SynkStatus::Ferdig);
    assert_eq!(ferdig_ny_row.antall_feil, 4);
    assert_eq!(avbrutt_gammel_row.status, SynkStatus::Avbrutt);
    assert_eq!(avbrutt_gammel_row.antall_feil, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_status_synk_id_tar_bare_egne_rader() {
    let pool = init_pool().await;

    let egen_synk_id = Uuid::new_v4();
    let mut egen_aktiv = TestData::from_status(SynkStatus::Aktiv);
    egen_aktiv.synk_id = egen_synk_id;
    let mut egen_ferdig = TestData::from_status(SynkStatus::Ferdig);
    egen_ferdig.synk_id = egen_synk_id;
    let annen_aktiv = TestData::from_status(SynkStatus::Aktiv);

    opprett_komplett(&pool, &egen_aktiv).await;
    opprett_komplett(&pool, &egen_ferdig).await;
    opprett_komplett(&pool, &annen_aktiv).await;

    let antall = oppdater_status(
        &pool,
        SynkStatus::Aktiv,
        SynkStatus::Venter,
        Betingelse::SynkId(egen_synk_id),
        false,
    )
    .await;

    let egen_aktiv_row = finn_tiltak_synk_by_periode_id(&pool, &egen_aktiv.periode_id).await;
    let egen_ferdig_row = finn_tiltak_synk_by_periode_id(&pool, &egen_ferdig.periode_id).await;
    let annen_aktiv_row = finn_tiltak_synk_by_periode_id(&pool, &annen_aktiv.periode_id).await;

    assert_eq!(antall, 1);
    assert_eq!(egen_aktiv_row.status, SynkStatus::Venter);
    assert_eq!(egen_ferdig_row.status, SynkStatus::Ferdig);
    assert_eq!(annen_aktiv_row.status, SynkStatus::Aktiv);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn update_resultat_med_synk_id_vakt() {
    let pool = init_pool().await;

    let synk_id = Uuid::new_v4();
    let mut aktiv_ok = TestData::from_status(SynkStatus::Aktiv);
    aktiv_ok.synk_id = synk_id;
    aktiv_ok.sist_kjoert = for_timer_siden(3);
    aktiv_ok.antall_feil = 4;
    let mut aktiv_feil = TestData::from_status(SynkStatus::Aktiv);
    aktiv_feil.synk_id = synk_id;
    aktiv_feil.sist_kjoert = for_timer_siden(3);
    aktiv_feil.antall_feil = 4;
    let mut venter_ok = TestData::from_status(SynkStatus::Venter);
    venter_ok.synk_id = synk_id;
    venter_ok.sist_kjoert = for_timer_siden(3);
    venter_ok.antall_feil = 0;

    opprett_komplett(&pool, &aktiv_ok).await;
    opprett_komplett(&pool, &aktiv_feil).await;
    opprett_komplett(&pool, &venter_ok).await;

    let aktiv_ok_count =
        oppdater_resultat(&pool, &aktiv_ok.periode_id, &synk_id, SynkStatus::Ferdig).await;
    let aktiv_feil_count_1 =
        oppdater_resultat(&pool, &aktiv_feil.periode_id, &synk_id, SynkStatus::Feilet).await;
    let aktiv_feil_count_2 = oppdater_resultat(
        &pool,
        &aktiv_feil.periode_id,
        &Uuid::new_v4(),
        SynkStatus::Feilet,
    )
    .await;
    let venter_ok_count =
        oppdater_resultat(&pool, &venter_ok.periode_id, &synk_id, SynkStatus::Ferdig).await;

    let aktiv_ok_row = finn_tiltak_synk_by_periode_id(&pool, &aktiv_ok.periode_id).await;
    let aktiv_feil_row = finn_tiltak_synk_by_periode_id(&pool, &aktiv_feil.periode_id).await;
    let venter_ok_row = finn_tiltak_synk_by_periode_id(&pool, &venter_ok.periode_id).await;

    assert_eq!(aktiv_ok_count, 1);
    assert_eq!(aktiv_feil_count_1, 1);
    assert_eq!(aktiv_feil_count_2, 0);
    assert_eq!(venter_ok_count, 0);

    assert_eq!(aktiv_ok_row.status, SynkStatus::Ferdig);
    assert_eq!(aktiv_ok_row.antall_feil, 0);
    assert!(aktiv_ok_row.sist_kjoert > for_timer_siden(3));
    assert_eq!(aktiv_feil_row.status, SynkStatus::Feilet);
    assert_eq!(aktiv_feil_row.antall_feil, 5);
    assert!(aktiv_feil_row.sist_kjoert > for_timer_siden(3));
    assert_eq!(venter_ok_row.status, SynkStatus::Venter);
    assert_eq!(venter_ok_row.antall_feil, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_tar_bare_venter_eldste_foerst_og_respekterer_grense() {
    let pool = init_pool().await;
    let venter_size: usize = 20;
    let batch_size: usize = 10;

    let mut ventende_periode_id = Vec::new();
    for i in 0..venter_size {
        let mut venter = TestData::from_status(SynkStatus::Venter);
        venter.sist_kjoert = for_timer_siden((venter_size - i) as i64);
        opprett_komplett(&pool, &venter).await;
        ventende_periode_id.push(venter.periode_id);
    }
    let ferdig = TestData::from_status(SynkStatus::Ferdig);
    let feilet = TestData::from_status(SynkStatus::Feilet);
    opprett_komplett(&pool, &ferdig).await;
    opprett_komplett(&pool, &feilet).await;

    let synk_id = Uuid::new_v4();
    let batch = finn_tiltak_synk_batch(&pool, &synk_id, batch_size as i64).await;

    let ventende_batch_periode_id: HashSet<Uuid> =
        ventende_periode_id[..batch_size].iter().copied().collect();
    let batch_periode_id: HashSet<Uuid> = batch.iter().map(|r| r.periode_id).collect();

    assert_eq!(batch_periode_id, ventende_batch_periode_id);
    for periode_id in &ventende_batch_periode_id {
        let row = finn_tiltak_synk_by_periode_id(&pool, periode_id).await;
        assert_eq!(row.synk_id, synk_id);
        assert_eq!(row.status, SynkStatus::Aktiv);
    }
    for periode_id in &ventende_periode_id[batch_size..] {
        let venter_row = finn_tiltak_synk_by_periode_id(&pool, &periode_id).await;
        assert_ne!(venter_row.synk_id, synk_id);
        assert_eq!(venter_row.status, SynkStatus::Venter);
    }
    let ferdig_row = finn_tiltak_synk_by_periode_id(&pool, &ferdig.periode_id).await;
    let feilet_row = finn_tiltak_synk_by_periode_id(&pool, &feilet.periode_id).await;
    assert_ne!(ferdig_row.synk_id, synk_id);
    assert_eq!(ferdig_row.status, SynkStatus::Ferdig);
    assert_ne!(feilet_row.synk_id, synk_id);
    assert_eq!(feilet_row.status, SynkStatus::Feilet);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn claim_hopper_over_rader_laast_av_ucommittet_claim() {
    let pool = init_pool().await;

    let mut ventende_periode_id = Vec::new();

    for i in 0..4 {
        let mut venter = TestData::from_status(SynkStatus::Venter);
        venter.sist_kjoert = for_timer_siden(4 - i);
        opprett_komplett(&pool, &venter).await;
        ventende_periode_id.push(venter.periode_id);
    }

    let synk_id_a = Uuid::new_v4();
    let synk_id_b = Uuid::new_v4();
    let ventende_foerste_periode_id: HashSet<Uuid> =
        ventende_periode_id[..2].iter().copied().collect();
    let ventende_siste_periode_id: HashSet<Uuid> =
        ventende_periode_id[2..].iter().copied().collect();

    let mut tx_a = pool.begin().await.unwrap();
    let batch_a = select_batch(&mut tx_a, &synk_id_a, 2).await.unwrap();

    let batch_a_periode_id: HashSet<Uuid> = batch_a.iter().map(|r| r.periode_id).collect();
    assert_eq!(batch_a_periode_id, ventende_foerste_periode_id);

    let batch_b = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        finn_tiltak_synk_batch(&pool, &synk_id_b, 4),
    )
    .await
    .expect("B skal ikke vente på radlåsene til A");

    let batch_b_periode_id: HashSet<Uuid> = batch_b.iter().map(|r| r.periode_id).collect();
    assert_eq!(batch_b_periode_id, ventende_siste_periode_id);

    tx_a.commit().await.unwrap();

    for periode_id in &ventende_periode_id[..2] {
        let row = finn_tiltak_synk_by_periode_id(&pool, periode_id).await;
        assert_eq!(row.synk_id, synk_id_a);
    }
    for periode_id in &ventende_periode_id[2..] {
        let row = finn_tiltak_synk_by_periode_id(&pool, periode_id).await;
        assert_eq!(row.synk_id, synk_id_b);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn samtidige_claims_gir_disjunkte_sett() {
    let pool = init_pool().await;

    let ventende = TestData::batch_of_venter(200);
    let ventende_periode_id: HashSet<Uuid> = ventende.iter().map(|r| r.periode_id).collect();

    for v in ventende.iter() {
        opprett_komplett(&pool, &v).await;
    }

    let mut handles = Vec::new();
    for _ in 0..8 {
        let pool = pool.clone();
        handles.push(tokio::spawn(async move {
            let mut claimet: Vec<Uuid> = Vec::new();
            for _ in 0..10 {
                let rows = finn_tiltak_synk_batch(&pool, &Uuid::new_v4(), 10).await;
                claimet.extend(rows.iter().map(|r| r.periode_id));
            }
            claimet
        }));
    }

    let mut union = HashSet::new();
    let mut totalt = 0;
    for handle in handles {
        let sett = handle.await.unwrap();
        totalt += sett.len();
        union.extend(sett);
    }

    assert_eq!(totalt, union.len(), "samme rad claimet av flere");
    assert_eq!(union, ventende_periode_id, "alle VENTER skal være claimet");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn status_statistikk_grupperer_per_status() {
    let pool = init_pool().await;

    let mut ferdig = TestData::from_status(SynkStatus::Ferdig);
    ferdig.sist_kjoert = for_timer_siden(1);
    let mut feilet_lite = TestData::from_antall_feil(4);
    feilet_lite.sist_kjoert = for_timer_siden(2);
    let mut feilet_mye = TestData::from_antall_feil(5);
    feilet_mye.sist_kjoert = for_timer_siden(3);
    for data in [&ferdig, &feilet_lite, &feilet_mye] {
        opprett_komplett(&pool, data).await;
    }

    let mut tx = pool.begin().await.unwrap();
    let mut rader = tiltak_synk::select_status_statistikk(&mut tx, 5)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    rader.sort_by_key(|rad| rad.antall);

    assert_eq!(rader.len(), 2);
    assert_eq!(rader[0].status, SynkStatus::Ferdig);
    assert_eq!(rader[0].antall, 1);
    assert_eq!(rader[0].vedvarende_feil, 0);
    assert!((rader[0].eldste_sist_kjoert - ferdig.sist_kjoert).abs() < Duration::milliseconds(1));
    assert_eq!(rader[1].status, SynkStatus::Feilet);
    assert_eq!(rader[1].antall, 2);
    assert_eq!(rader[1].vedvarende_feil, 1);
    assert!(
        (rader[1].eldste_sist_kjoert - feilet_mye.sist_kjoert).abs() < Duration::milliseconds(1)
    );
}
