use std::num::NonZeroU16;
use std::sync::Arc;

use anyhow::Result;
use chrono::Duration;
use mockito::{Server, ServerGuard};
use paw_test::setup_test_db::{TestDbGuard, setup_test_db};
use paw_test::stub_token_client::StubTokenClient;
use serde_json::json;
use sqlx::PgPool;
use utgang::kafka::periode_deserializer::{Bruker, BrukerType, Metadata, Periode};
use utgang::pdl::pdl_query::PDLClient;
use utgang::pdl_oppdatering::PdlDataOppdatering;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct TestPeriodeRow {
    pub arbeidssoeker_id: Option<i64>,
    pub sist_oppdatert: chrono::NaiveDateTime,
    pub trenger_kontroll: bool,
    pub stoppet: Option<serde_json::Value>,
    pub tilstand: Option<serde_json::Value>,
    pub bekreftet: bool,
}

pub async fn setup() -> PgPool {
    let (pool, _guard) = setup_test_db()
        .await
        .expect("Failed to setup test database");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    pool
}

pub async fn les_rad(pool: &PgPool, id: Uuid) -> Option<TestPeriodeRow> {
    sqlx::query_as(
        "SELECT arbeidssoeker_id, sist_oppdatert, trenger_kontroll, stoppet, tilstand, bekreftet \
         FROM perioder WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .unwrap()
}

fn test_metadata(tidspunkt: chrono::DateTime<chrono::Utc>) -> Metadata {
    Metadata {
        tidspunkt,
        utfoert_av: Bruker {
            bruker_type: BrukerType::Sluttbruker,
            id: "12345678901".to_string(),
            sikkerhetsnivaa: None,
        },
        kilde: "test".to_string(),
        aarsak: "test".to_string(),
        tidspunkt_fra_kilde: None,
    }
}

pub fn aktiv_periode(id: Uuid) -> Periode {
    use paw_test::hendelse_builder::rfc3339;
    Periode {
        id,
        identitetsnummer: "12345678901".to_string(),
        startet: test_metadata(rfc3339("2024-01-01T00:00:00Z")),
        avsluttet: None,
    }
}

pub fn avsluttet_periode(id: Uuid) -> Periode {
    use paw_test::hendelse_builder::rfc3339;
    Periode {
        id,
        identitetsnummer: "12345678901".to_string(),
        startet: test_metadata(rfc3339("2024-01-01T00:00:00Z")),
        avsluttet: Some(test_metadata(rfc3339("2024-06-01T00:00:00Z"))),
    }
}

pub struct PdlTestContext {
    pub pool: PgPool,
    _guard: TestDbGuard,
    pub pdl_server: ServerGuard,
}

impl PdlTestContext {
    pub async fn ny() -> Result<Self> {
        let (pool, _guard) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let pdl_server = Server::new_async().await;
        Ok(Self { pool, _guard, pdl_server })
    }

    pub fn pdl_oppdatering(&self, data_gyldighet: Duration) -> PdlDataOppdatering {
        let pdl_client = PDLClient::new(
            "test-scope".to_string(),
            self.pdl_server.url(),
            reqwest::Client::new(),
            Arc::new(StubTokenClient),
        );
        PdlDataOppdatering::new(self.pool.clone(), pdl_client, NonZeroU16::new(100).unwrap(), data_gyldighet)
    }

    pub async fn stub_pdl_med_person(&mut self, identer: &[&str]) -> mockito::Mock {
        let bolk: Vec<serde_json::Value> = identer
            .iter()
            .map(|ident| json!({
                "ident": ident,
                "person": {
                    "foedselsdato": [], "statsborgerskap": [], "opphold": [],
                    "folkeregisterpersonstatus": [], "bostedsadresse": [],
                    "innflyttingTilNorge": [], "utflyttingFraNorge": []
                },
                "code": "ok"
            }))
            .collect();
        self.pdl_server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({ "data": { "hentPersonBolk": bolk } }).to_string())
            .create_async()
            .await
    }

    pub async fn stub_pdl_uten_person(&mut self, ident: &str) -> mockito::Mock {
        self.pdl_server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(json!({
                "data": { "hentPersonBolk": [{ "ident": ident, "person": null, "code": "not_found" }] }
            }).to_string())
            .create_async()
            .await
    }

    pub async fn les_rad(&self, id: Uuid) -> Option<(chrono::NaiveDateTime, bool)> {
        sqlx::query_as("SELECT sist_oppdatert, trenger_kontroll FROM perioder WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
    }
}
