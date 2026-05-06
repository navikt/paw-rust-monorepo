use chrono::Timelike;
use std::collections::HashSet;

use chrono::{Duration, Utc};
use interne_hendelser::vo::{BrukerType, Opplysning};
use paw_test::setup_test_db::setup_test_db;
use utgang::dao::utgang_hendelse::{Input, InternUtgangHendelse};
use utgang::dao::utgang_hendelser_logg::{hent_hendelser, hent_metadata_og_siste_pdl, skriv_hendelser};
use utgang::domain::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use utgang::domain::opplysninger::Opplysninger;
use utgang::domain::utgang_hendelse_type::UtgangHendelseType;
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

fn lag_hendelse(
    periode_id: ArbeidssoekerperiodeId,
    timestamp: chrono::DateTime<Utc>,
    opplysninger: Option<HashSet<Opplysning>>,
) -> InternUtgangHendelse<Input> {
    InternUtgangHendelse::new(
        UtgangHendelseType::StatusEndretTilOK,
        periode_id,
        timestamp,
        BrukerType::Sluttbruker,
        opplysninger.map(Opplysninger),
    )
}

#[tokio::test]
async fn skriv_og_hent_roundtrip_bevarer_alle_felter() {
    let pool = setup().await;

    let periode_id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let timestamp = Utc::now().with_nanosecond_truncated();
    let opplysninger = HashSet::from([Opplysning::ErOver18Aar, Opplysning::HarNorskAdresse]);

    let hendelse = InternUtgangHendelse::new(
        UtgangHendelseType::StatusEndretTilAvvist,
        periode_id.clone(),
        timestamp,
        BrukerType::Veileder,
        Some(Opplysninger(opplysninger.clone())),
    );

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(&mut tx, vec![hendelse])
        .await
        .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_hendelser(&mut tx, &[periode_id.clone()])
        .await
        .expect("hent_hendelser feilet");
    tx.commit().await.unwrap();

    let hendelser = resultat
        .get(&periode_id)
        .expect("Periode-ID ikke funnet i resultat");
    assert_eq!(hendelser.len(), 1);

    let h = &hendelser[0];
    assert!(matches!(
        h.hendelsetype(),
        UtgangHendelseType::StatusEndretTilAvvist
    ));
    assert_eq!(h.periode_id(), &periode_id);
    assert_eq!(h.timestamp(), timestamp);
    assert!(matches!(h.brukertype(), BrukerType::Veileder));
    let lest_opplysninger: HashSet<Opplysning> =
        h.opplysninger().expect("Forventet opplysninger").0.clone();
    assert_eq!(lest_opplysninger, opplysninger);
    let _ = h.primary_key(); // Verifiser at primary key er satt (ikke panikk)
}

#[tokio::test]
async fn hent_hendelser_returnerer_kun_forespurte_periode_ider() {
    let pool = setup().await;

    let periode_a = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let periode_b = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now();

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(
        &mut tx,
        vec![
            lag_hendelse(periode_a.clone(), now, None),
            lag_hendelse(periode_b.clone(), now, None),
        ],
    )
    .await
    .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_hendelser(&mut tx, &[periode_a.clone()])
        .await
        .expect("hent_hendelser feilet");
    tx.commit().await.unwrap();

    assert!(
        resultat.contains_key(&periode_a),
        "Periode A skal være i resultatet"
    );
    assert!(
        !resultat.contains_key(&periode_b),
        "Periode B skal ikke være i resultatet"
    );
}

#[tokio::test]
async fn hent_hendelser_returnerer_i_stigende_timestamp_rekkefølge() {
    let pool = setup().await;

    let periode_id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now().with_nanosecond_truncated();
    let tidlig = (now - Duration::minutes(10)).with_nanosecond_truncated();
    let midtre = (now - Duration::minutes(5)).with_nanosecond_truncated();
    let sen = now;

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(
        &mut tx,
        vec![
            lag_hendelse(periode_id.clone(), sen, None),
            lag_hendelse(periode_id.clone(), tidlig, None),
            lag_hendelse(periode_id.clone(), midtre, None),
        ],
    )
    .await
    .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_hendelser(&mut tx, &[periode_id.clone()])
        .await
        .expect("hent_hendelser feilet");
    tx.commit().await.unwrap();

    let hendelser = resultat.get(&periode_id).expect("Periode-ID ikke funnet");
    assert_eq!(hendelser.len(), 3);
    assert_eq!(hendelser[0].timestamp(), tidlig);
    assert_eq!(hendelser[1].timestamp(), midtre);
    assert_eq!(hendelser[2].timestamp(), sen);
}

#[tokio::test]
async fn hent_hendelser_returnerer_tom_for_ukjent_periode_id() {
    let pool = setup().await;

    let ukjent = ArbeidssoekerperiodeId::from(Uuid::new_v4());

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_hendelser(&mut tx, &[ukjent.clone()])
        .await
        .expect("hent_hendelser feilet");
    tx.commit().await.unwrap();

    assert!(
        resultat.get(&ukjent).is_none_or(|v| v.is_empty()),
        "Ukjent periode-ID skal ikke gi resultater"
    );
}

#[tokio::test]
async fn hent_hendelser_tom_input_returnerer_tomt_resultat() {
    let pool = setup().await;

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_hendelser(&mut tx, &[])
        .await
        .expect("hent_hendelser feilet");
    tx.commit().await.unwrap();

    assert!(resultat.is_empty());
}

#[tokio::test]
async fn hent_hendelser_uten_opplysninger_returnerer_none() {
    let pool = setup().await;

    let periode_id = ArbeidssoekerperiodeId::from(Uuid::new_v4());

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(
        &mut tx,
        vec![lag_hendelse(periode_id.clone(), Utc::now(), None)],
    )
    .await
    .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_hendelser(&mut tx, &[periode_id.clone()])
        .await
        .expect("hent_hendelser feilet");
    tx.commit().await.unwrap();

    let h = &resultat[&periode_id][0];
    assert!(h.opplysninger().is_none(), "Opplysninger skal være None");
}

#[tokio::test]
async fn hent_metadata_og_siste_pdl_returnerer_metadata_mottatt_og_siste_pdl() {
    let pool = setup().await;

    let periode_id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now().with_nanosecond_truncated();
    let tidlig = (now - Duration::minutes(10)).with_nanosecond_truncated();

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(
        &mut tx,
        vec![
            InternUtgangHendelse::new(
                UtgangHendelseType::MetadataMottatt,
                periode_id.clone(),
                tidlig,
                BrukerType::Sluttbruker,
                None,
            ),
            InternUtgangHendelse::new(
                UtgangHendelseType::PdlDataEndret,
                periode_id.clone(),
                now - Duration::minutes(5),
                BrukerType::Sluttbruker,
                None,
            ),
            InternUtgangHendelse::new(
                UtgangHendelseType::PdlDataEndret,
                periode_id.clone(),
                now,
                BrukerType::Sluttbruker,
                None,
            ),
        ],
    )
    .await
    .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_metadata_og_siste_pdl(&mut tx, &[periode_id.clone()])
        .await
        .expect("hent_metadata_og_siste_pdl feilet");
    tx.commit().await.unwrap();

    let data = resultat.get(&periode_id).expect("Periode-ID ikke funnet");
    assert!(matches!(
        data.metadata_mottatt.hendelsetype(),
        UtgangHendelseType::MetadataMottatt
    ));
    assert_eq!(data.metadata_mottatt.timestamp(), tidlig);
    let siste = data.siste_pdl_data_endret.as_ref().expect("Forventet PdlDataEndret");
    assert_eq!(siste.timestamp(), now);
}

#[tokio::test]
async fn hent_metadata_og_siste_pdl_uten_pdl_data_endret() {
    let pool = setup().await;

    let periode_id = ArbeidssoekerperiodeId::from(Uuid::new_v4());
    let now = Utc::now().with_nanosecond_truncated();

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(
        &mut tx,
        vec![InternUtgangHendelse::new(
            UtgangHendelseType::MetadataMottatt,
            periode_id.clone(),
            now,
            BrukerType::Sluttbruker,
            None,
        )],
    )
    .await
    .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_metadata_og_siste_pdl(&mut tx, &[periode_id.clone()])
        .await
        .expect("hent_metadata_og_siste_pdl feilet");
    tx.commit().await.unwrap();

    let data = resultat.get(&periode_id).expect("Periode-ID ikke funnet");
    assert!(data.siste_pdl_data_endret.is_none());
}

#[tokio::test]
async fn hent_metadata_og_siste_pdl_uten_metadata_mottatt_returneres_ikke() {
    let pool = setup().await;

    let periode_id = ArbeidssoekerperiodeId::from(Uuid::new_v4());

    let mut tx = pool.begin().await.unwrap();
    skriv_hendelser(
        &mut tx,
        vec![InternUtgangHendelse::new(
            UtgangHendelseType::PdlDataEndret,
            periode_id.clone(),
            Utc::now(),
            BrukerType::Sluttbruker,
            None,
        )],
    )
    .await
    .expect("skriv_hendelser feilet");
    tx.commit().await.unwrap();

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_metadata_og_siste_pdl(&mut tx, &[periode_id.clone()])
        .await
        .expect("hent_metadata_og_siste_pdl feilet");
    tx.commit().await.unwrap();

    assert!(
        resultat.get(&periode_id).is_none(),
        "Periode uten MetadataMottatt skal ikke returneres"
    );
}

#[tokio::test]
async fn hent_metadata_og_siste_pdl_tom_input_returnerer_tomt_resultat() {
    let pool = setup().await;

    let mut tx = pool.begin().await.unwrap();
    let resultat = hent_metadata_og_siste_pdl(&mut tx, &[])
        .await
        .expect("hent_metadata_og_siste_pdl feilet");
    tx.commit().await.unwrap();

    assert!(resultat.is_empty());
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
