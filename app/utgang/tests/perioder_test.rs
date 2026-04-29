use chrono::{Duration, Timelike, Utc};
use paw_test::setup_test_db::setup_test_db;
use std::num::NonZeroU32;
use utgang::dao::perioder::{
    hent_perioder, hent_perioder_eldre_enn, hent_perioder_som_trenger_kontroll,
    oppdater_sist_oppdatert, oppdater_stoppet, oppdater_trenger_kontroll, skriv_perioder,
    PeriodeRad,
};
use utgang::domain::arbeidssoeker_id::ArbeidssoekerId;
use utgang::domain::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use uuid::Uuid;

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

fn lag_periode(
    id: ArbeidssoekerperiodeId,
    arbeidssoeker_id: Option<i64>,
    trenger_kontroll: bool,
    stoppet: bool,
    sist_oppdatert: chrono::DateTime<Utc>,
) -> PeriodeRad {
    PeriodeRad {
        id,
        arbeidssoeker_id: arbeidssoeker_id.map(ArbeidssoekerId),
        trenger_kontroll,
        stoppet,
        sist_oppdatert,
    }
}

trait TruncateNanoseconds {
    fn with_nanosecond_truncated(self) -> Self;
}

impl TruncateNanoseconds for chrono::DateTime<Utc> {
    fn with_nanosecond_truncated(self) -> Self {
        self.with_nanosecond(self.nanosecond() / 1_000_000 * 1_000_000)
            .unwrap()
    }
}

#[tokio::test]
async fn skriv_og_hent_roundtrip_bevarer_alle_felter() {
    let pool = setup().await;

    let id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let sist_oppdatert = Utc::now().with_nanosecond_truncated();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), Some(42), false, false, sist_oppdatert)])
        .await
        .expect("skriv_perioder feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id.clone()])
        .await
        .expect("hent_perioder feilet");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    let rad = &rader[0];
    assert_eq!(rad.id, id);
    assert_eq!(rad.arbeidssoeker_id.as_ref().map(|a| a.0), Some(42));
    assert!(!rad.trenger_kontroll);
    assert!(!rad.stoppet);
    assert_eq!(rad.sist_oppdatert, sist_oppdatert);
}

#[tokio::test]
async fn skriv_og_hent_roundtrip_uten_arbeidssoeker_id() {
    let pool = setup().await;

    let id = ArbeidssoekerperiodeId::from(Uuid::new_v4());

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), None, true, false, Utc::now())])
        .await
        .expect("skriv_perioder feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id.clone()])
        .await
        .expect("hent_perioder feilet");
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert!(rader[0].arbeidssoeker_id.is_none());
}

#[tokio::test]
async fn skriv_perioder_upsert_oppdaterer_eksisterende() {
    let pool = setup().await;

    let id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let opprinnelig = Utc::now().with_nanosecond_truncated();
    let oppdatert = (opprinnelig + Duration::minutes(5)).with_nanosecond_truncated();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), Some(1), false, false, opprinnelig)])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), Some(2), true, false, oppdatert)])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id.clone()]).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    let rad = &rader[0];
    assert_eq!(rad.arbeidssoeker_id.as_ref().map(|a| a.0), Some(2));
    assert!(rad.trenger_kontroll);
    assert_eq!(rad.sist_oppdatert, oppdatert);
}

#[tokio::test]
async fn hent_perioder_returnerer_kun_forespurte_ider() {
    let pool = setup().await;

    let id_a = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let id_b = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(id_a.clone(), None, false, false, now),
            lag_periode(id_b.clone(), None, false, false, now),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id_a.clone()]).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert_eq!(rader[0].id, id_a);
}

#[tokio::test]
async fn hent_perioder_tom_input_returnerer_tom_liste() {
    let pool = setup().await;

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[]).await.unwrap();
    tx.commit().await.unwrap();

    assert!(rader.is_empty());
}

#[tokio::test]
async fn hent_perioder_eldre_enn_returnerer_kun_rader_foer_grense() {
    let pool = setup().await;

    let grense = Utc::now();
    let gammel = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let ny = ArbeidssoekerperiodeId::from(Uuid::new_v4());

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(gammel.clone(), None, false, false, grense - Duration::minutes(10)),
            lag_periode(ny.clone(), None, false, false, grense + Duration::minutes(10)),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU32::new(10).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert_eq!(rader[0].id, gammel);
}

#[tokio::test]
async fn hent_perioder_eldre_enn_ignorerer_rader_med_trenger_kontroll_true() {
    let pool = setup().await;

    let grense = Utc::now();
    let gammel_uten_kontroll = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let gammel_med_kontroll = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let tidspunkt = grense - Duration::minutes(10);

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(gammel_uten_kontroll.clone(), None, false, false, tidspunkt),
            lag_periode(gammel_med_kontroll.clone(), None, true, false, tidspunkt),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU32::new(10).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert_eq!(rader[0].id, gammel_uten_kontroll);
}

#[tokio::test]
async fn hent_perioder_eldre_enn_returnerer_i_stigende_rekkefølge() {
    let pool = setup().await;

    let grense = Utc::now();
    let tidlig = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let sen = ArbeidssoekerperiodeId::from(Uuid::new_v4());

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(sen.clone(), None, false, false, grense - Duration::minutes(1)),
            lag_periode(tidlig.clone(), None, false, false, grense - Duration::minutes(10)),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU32::new(10).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader[0].id, tidlig);
    assert_eq!(rader[1].id, sen);
}

#[tokio::test]
async fn hent_perioder_eldre_enn_respekterer_limit() {
    let pool = setup().await;

    let grense = Utc::now();
    let tidspunkt = grense - Duration::minutes(10);

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(ArbeidssoekerperiodeId::from(Uuid::new_v4()), None, false, false, tidspunkt),
            lag_periode(ArbeidssoekerperiodeId::from(Uuid::new_v4()), None, false, false, tidspunkt),
            lag_periode(ArbeidssoekerperiodeId::from(Uuid::new_v4()), None, false, false, tidspunkt),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU32::new(2).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 2);
}

#[tokio::test]
async fn hent_perioder_som_trenger_kontroll_returnerer_kun_true_rader() {
    let pool = setup().await;

    let med = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let uten = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(med.clone(), None, true, false, now),
            lag_periode(uten.clone(), None, false, false, now),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_som_trenger_kontroll(&mut tx, NonZeroU32::new(10).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1);
    assert_eq!(rader[0].id, med);
    assert!(rader[0].trenger_kontroll);
}

#[tokio::test]
async fn hent_perioder_som_trenger_kontroll_respekterer_limit() {
    let pool = setup().await;
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(ArbeidssoekerperiodeId::from(Uuid::new_v4()), None, true, false, now),
            lag_periode(ArbeidssoekerperiodeId::from(Uuid::new_v4()), None, true, false, now),
            lag_periode(ArbeidssoekerperiodeId::from(Uuid::new_v4()), None, true, false, now),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_som_trenger_kontroll(&mut tx, NonZeroU32::new(2).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 2);
}

#[tokio::test]
async fn oppdater_trenger_kontroll_oppdaterer_kun_angitte_ider() {
    let pool = setup().await;

    let id_a = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let id_b = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(id_a.clone(), None, false, false, now),
            lag_periode(id_b.clone(), None, false, false, now),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    oppdater_trenger_kontroll(&mut tx, &[id_a.clone()], true)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id_a.clone(), id_b.clone()])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let rad_a = rader.iter().find(|r| r.id == id_a).unwrap();
    let rad_b = rader.iter().find(|r| r.id == id_b).unwrap();
    assert!(rad_a.trenger_kontroll, "id_a skal ha trenger_kontroll=true");
    assert!(!rad_b.trenger_kontroll, "id_b skal ikke være endret");
}

#[tokio::test]
async fn oppdater_sist_oppdatert_oppdaterer_kun_angitte_ider() {
    let pool = setup().await;

    let id_a = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let id_b = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let opprinnelig = Utc::now().with_nanosecond_truncated();
    let ny_tid = (opprinnelig + Duration::hours(1)).with_nanosecond_truncated();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(id_a.clone(), None, false, false, opprinnelig),
            lag_periode(id_b.clone(), None, false, false, opprinnelig),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    oppdater_sist_oppdatert(&mut tx, &[id_a.clone()], ny_tid)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id_a.clone(), id_b.clone()])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let rad_a = rader.iter().find(|r| r.id == id_a).unwrap();
    let rad_b = rader.iter().find(|r| r.id == id_b).unwrap();
    assert_eq!(rad_a.sist_oppdatert, ny_tid, "id_a skal ha ny tid");
    assert_eq!(rad_b.sist_oppdatert, opprinnelig, "id_b skal ikke være endret");
}

#[tokio::test]
async fn oppdater_stoppet_setter_flagg_og_ekskluderer_fra_hent() {
    let pool = setup().await;

    let id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), None, false, false, now)])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    oppdater_stoppet(&mut tx, &[id.clone()]).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id.clone()]).await.unwrap();
    tx.commit().await.unwrap();

    assert!(rader.is_empty(), "stoppet periode skal ikke returneres av hent_perioder");
}

#[tokio::test]
async fn stoppet_periode_ekskluderes_fra_eldre_enn() {
    let pool = setup().await;

    let id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let grense = Utc::now();
    let tidspunkt = grense - Duration::minutes(10);

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), None, false, true, tidspunkt)])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_eldre_enn(&mut tx, grense, NonZeroU32::new(10).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert!(rader.is_empty(), "stoppet periode skal ikke returneres av hent_perioder_eldre_enn");
}

#[tokio::test]
async fn stoppet_periode_ekskluderes_fra_trenger_kontroll() {
    let pool = setup().await;

    let id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(&mut tx, vec![lag_periode(id.clone(), None, true, true, now)])
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder_som_trenger_kontroll(&mut tx, NonZeroU32::new(10).unwrap())
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert!(
        rader.is_empty(),
        "stoppet periode skal ikke returneres selv om trenger_kontroll=true"
    );
}

#[tokio::test]
async fn oppdater_stoppet_pavirker_kun_angitte_ider() {
    let pool = setup().await;

    let id_a = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let id_b = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_perioder(
        &mut tx,
        vec![
            lag_periode(id_a.clone(), None, false, false, now),
            lag_periode(id_b.clone(), None, false, false, now),
        ],
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    oppdater_stoppet(&mut tx, &[id_a.clone()]).await.unwrap();
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let rader = hent_perioder(&mut tx, &[id_b.clone()]).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(rader.len(), 1, "id_b er ikke stoppet og skal fremdeles returneres");
    assert_eq!(rader[0].id, id_b);
}
