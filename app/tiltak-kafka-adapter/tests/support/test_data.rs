use chrono::{DateTime, Utc};
use postgres_testcontainer::postgres::setup_postgres_container;
use sqlx::{PgPool, Postgres, Transaction};
use tiltak_kafka_adapter::model::dao::arbeidssoeker::ArbeidssoekerRow;
use tiltak_kafka_adapter::model::dao::tiltak_synk::{
    BatchTiltakSynkRow, Betingelse, TiltakSynkRow, select_batch,
};
use tiltak_kafka_adapter::model::dao::{arbeidssoeker, tiltak_synk};
use tiltak_kafka_adapter::model::status::SynkStatus;
use uuid::Uuid;

pub const IDENTITETSNUMMER: &str = "01017012345";

#[derive(Debug, Clone)]
pub struct TestData {
    pub periode_id: Uuid,
    pub identitetsnummer: String,
    pub synk_id: Uuid,
    pub status: SynkStatus,
    pub status_endret: DateTime<Utc>,
    pub sist_kjoert: DateTime<Utc>,
    pub antall_feil: i32,
}

impl TestData {
    pub fn from_scratch() -> Self {
        Self {
            periode_id: Uuid::new_v4(),
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            synk_id: Uuid::new_v4(),
            status: SynkStatus::Venter,
            status_endret: Utc::now(),
            sist_kjoert: Utc::now(),
            antall_feil: 0,
        }
    }

    pub fn from_identitetsnummer(identitetsnummer: &str) -> Self {
        Self {
            periode_id: Uuid::new_v4(),
            identitetsnummer: identitetsnummer.to_string(),
            synk_id: Uuid::new_v4(),
            status: SynkStatus::Venter,
            status_endret: Utc::now(),
            sist_kjoert: Utc::now(),
            antall_feil: 0,
        }
    }

    pub fn from_status(status: SynkStatus) -> Self {
        Self {
            periode_id: Uuid::new_v4(),
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            synk_id: Uuid::new_v4(),
            status,
            status_endret: Utc::now(),
            sist_kjoert: Utc::now(),
            antall_feil: 0,
        }
    }

    pub fn from_antall_feil(antall_feil: i32) -> Self {
        Self {
            periode_id: Uuid::new_v4(),
            identitetsnummer: IDENTITETSNUMMER.to_string(),
            synk_id: Uuid::new_v4(),
            status: SynkStatus::Feilet,
            status_endret: Utc::now(),
            sist_kjoert: Utc::now(),
            antall_feil,
        }
    }

    pub fn batch_of_venter(count: i32) -> Vec<Self> {
        let mut batch: Vec<TestData> = Vec::new();
        for _ in 0..count {
            batch.push(Self::from_status(SynkStatus::Venter));
        }
        batch
    }
}

pub async fn init_pool() -> PgPool {
    let postgres_guard = setup_postgres_container()
        .await
        .expect("Failed to start Postgres container");
    sqlx::migrate!("./migrations")
        .run(&postgres_guard.pg_pool)
        .await
        .expect("Failed to run migrations");
    postgres_guard.pg_pool
}

async fn insert_arbeidssoeker(tx: &mut Transaction<'_, Postgres>, test_data: &TestData) {
    let row = ArbeidssoekerRow {
        periode_id: test_data.periode_id.clone(),
        identitetsnummer: test_data.identitetsnummer.clone(),
        periode_startet: Utc::now(),
    };
    arbeidssoeker::insert(tx, &row)
        .await
        .expect("Kunne ikke lagre arbeidssoeker");
}

async fn insert_tiltak_synk(tx: &mut Transaction<'_, Postgres>, test_data: &TestData) {
    let row = TiltakSynkRow {
        periode_id: test_data.periode_id.clone(),
        synk_id: test_data.synk_id.clone(),
        status: test_data.status.clone(),
        status_endret: test_data.status_endret.clone(),
        sist_kjoert: test_data.sist_kjoert.clone(),
        antall_feil: test_data.antall_feil.clone(),
    };
    tiltak_synk::insert(tx, &row)
        .await
        .expect("Kunne ikke lagre arbeidssoeker");
}

pub async fn finn_tiltak_synk_by_periode_id(pool: &PgPool, periode_id: &Uuid) -> TiltakSynkRow {
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    let row = tiltak_synk::select_by_periode_id(&mut tx, &periode_id)
        .await
        .expect("Kunne ikke finne rad");
    tx.commit().await.expect("Kunne ikke commit transaksjon");
    row.expect("Ingen rad funnet")
}

pub async fn count_tiltak_synk_by_status(pool: &PgPool, status: &SynkStatus) -> i64 {
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    let count = tiltak_synk::count_by_status(&mut tx, &status)
        .await
        .expect("Kunne ikke finne rad");
    tx.commit().await.expect("Kunne ikke commit transaksjon");
    count
}

pub async fn finn_tiltak_synk_batch(
    pool: &PgPool,
    synk_id: &Uuid,
    batch_stoerrelse: i64,
) -> Vec<BatchTiltakSynkRow> {
    let mut tx = pool.begin().await.unwrap();
    let rader = select_batch(&mut tx, synk_id, batch_stoerrelse)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    rader
}

pub async fn opprett_arbeidssoeker(pool: &PgPool, test_data: &TestData) {
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    insert_arbeidssoeker(&mut tx, test_data).await;
    tx.commit().await.expect("Kunne ikke commit transaksjon");
}

pub async fn opprett_arbeidssoeker_batch(
    pool: &PgPool,
    antall: usize,
    status: SynkStatus,
) -> Vec<Uuid> {
    let mut data = Vec::new();
    for _ in 0..antall {
        data.push(TestData::from_status(status.clone()));
    }
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    for d in &data {
        insert_arbeidssoeker(&mut tx, d).await;
    }
    tx.commit().await.expect("Kunne ikke commit transaksjon");
    data.iter().map(|d| d.periode_id.clone()).collect()
}

pub async fn opprett_komplett(pool: &PgPool, test_data: &TestData) {
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    insert_arbeidssoeker(&mut tx, test_data).await;
    insert_tiltak_synk(&mut tx, test_data).await;
    tx.commit().await.expect("Kunne ikke commit transaksjon");
}

pub async fn opprett_komplett_batch(pool: &PgPool, antall: usize, status: SynkStatus) -> Vec<Uuid> {
    let mut data = Vec::new();
    for _ in 0..antall {
        data.push(TestData::from_status(status.clone()));
    }
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    for d in &data {
        insert_arbeidssoeker(&mut tx, d).await;
        insert_tiltak_synk(&mut tx, d).await;
    }
    tx.commit().await.expect("Kunne ikke commit transaksjon");
    data.iter().map(|d| d.periode_id.clone()).collect()
}

pub async fn oppdater_status(
    pool: &PgPool,
    fra: SynkStatus,
    til: SynkStatus,
    betingelse: Betingelse,
    nullstill_feil: bool,
) -> u64 {
    let mut tx = pool.begin().await.expect("Kunne ikke starte transaksjon");
    let antall = tiltak_synk::update_status(&mut tx, &fra, &til, &betingelse, nullstill_feil)
        .await
        .expect("Kunne ikke oppdatere status");
    tx.commit().await.expect("Kunne ikke commit transaksjon");
    antall
}

pub async fn oppdater_resultat(
    pool: &PgPool,
    periode_id: &Uuid,
    synk_id: &Uuid,
    til: SynkStatus,
) -> u64 {
    let mut tx = pool.begin().await.unwrap();
    let antall = tiltak_synk::update_resultat(&mut tx, periode_id, synk_id, &til)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    antall
}

pub fn for_timer_siden(timer: i64) -> DateTime<Utc> {
    Utc::now() - chrono::Duration::hours(timer)
}
