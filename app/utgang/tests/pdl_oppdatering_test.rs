use std::num::NonZeroU16;
use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, Timelike, Utc};
use interne_hendelser::vo::Opplysning;
use mockito::{Server, ServerGuard};
use paw_test::hendelse_builder::StartetBuilder;
use paw_test::setup_test_db::{TestDbGuard, setup_test_db};
use paw_test::stub_token_client::StubTokenClient;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use utgang::dao::skriv_periode::skriv_startet_hendelse;
use utgang::pdl::pdl_query::PDLClient;
use utgang::pdl_oppdatering::PdlDataOppdatering;
use uuid::Uuid;

struct TestContext {
    pool: PgPool,
    _guard: TestDbGuard,
    pdl_server: ServerGuard,
}

impl TestContext {
    async fn ny() -> Result<Self> {
        let (pool, _guard) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let pdl_server = Server::new_async().await;
        Ok(Self {
            pool,
            _guard,
            pdl_server,
        })
    }

    fn pdl_oppdatering(&self, data_gyldighet: Duration) -> PdlDataOppdatering {
        let pdl_client = PDLClient::new(
            "test-scope".to_string(),
            self.pdl_server.url(),
            reqwest::Client::new(),
            Arc::new(StubTokenClient),
        );
        PdlDataOppdatering::new(
            self.pool.clone(),
            pdl_client,
            NonZeroU16::new(100).unwrap(),
            data_gyldighet,
        )
    }

    async fn stub_pdl_respons(&mut self, ident: &str) -> mockito::Mock {
        let respons = lag_pdl_respons(ident);
        self.pdl_server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(respons.to_string())
            .create_async()
            .await
    }

    async fn stub_pdl_uten_person(&mut self, ident: &str) -> mockito::Mock {
        let respons = lag_pdl_respons_uten_person(ident);
        self.pdl_server
            .mock("POST", "/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(respons.to_string())
            .create_async()
            .await
    }

    async fn les_rad(&self, id: Uuid) -> Option<(chrono::NaiveDateTime, bool)> {
        sqlx::query_as("SELECT sist_oppdatert, trenger_kontroll FROM perioder WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
    }
}

fn lag_pdl_respons(ident: &str) -> serde_json::Value {
    json!({
        "data": {
            "hentPersonBolk": [{
                "ident": ident,
                "person": {
                    "foedselsdato": [],
                    "statsborgerskap": [],
                    "opphold": [],
                    "folkeregisterpersonstatus": [],
                    "bostedsadresse": [],
                    "innflyttingTilNorge": [],
                    "utflyttingFraNorge": []
                },
                "code": "ok"
            }]
        }
    })
}

fn lag_pdl_respons_uten_person(ident: &str) -> serde_json::Value {
    json!({
        "data": {
            "hentPersonBolk": [{
                "ident": ident,
                "person": null,
                "code": "not_found"
            }]
        }
    })
}

/// Skriver en startet hendelse med gammelt tidspunkt slik at den plukkes opp av PDL-oppdatering.
async fn skriv_gammel_periode(pool: &PgPool, id: Uuid, ident: &str) {
    use paw_test::hendelse_builder::rfc3339;
    let startet = StartetBuilder {
        hendelse_id: id,
        identitetsnummer: ident.to_string(),
        utfoert_av_id: ident.to_string(),
        tidspunkt: rfc3339("2024-01-01T00:00:00Z"),
        opplysninger: HashSet::from([Opplysning::ErOver18Aar]),
        ..Default::default()
    }
    .build();
    let mut tx = pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();
}

// --- kjoer_oppdatering ---

#[tokio::test]
async fn kjoer_oppdatering_oppdaterer_sist_oppdatert_og_setter_trenger_kontroll() -> Result<()> {
    let mut ctx = TestContext::ny().await?;
    let ident = "12345678901";
    let periode_id = Uuid::new_v4();

    skriv_gammel_periode(&ctx.pool, periode_id, ident).await;
    let _mock = ctx.stub_pdl_respons(ident).await;

    let gjeldene = Utc::now()
        .with_nanosecond(0)
        .unwrap()
        .checked_add_signed(Duration::hours(2))
        .unwrap();
    let hadde_arbeid = ctx
        .pdl_oppdatering(Duration::hours(1))
        .kjoer_oppdatering(gjeldene)
        .await?;

    assert!(hadde_arbeid, "Skal returnere true når en periode ble oppdatert");

    let (sist_oppdatert, trenger_kontroll) = ctx
        .les_rad(periode_id)
        .await
        .expect("Perioden skal finnes");
    assert!(trenger_kontroll, "trenger_kontroll skal settes til true");
    assert_eq!(
        sist_oppdatert,
        gjeldene.naive_utc(),
        "sist_oppdatert skal settes til gjeldene_tidspunkt"
    );

    Ok(())
}

#[tokio::test]
async fn kjoer_oppdatering_returnerer_false_naar_ingen_utdaterte_perioder() -> Result<()> {
    let ctx = TestContext::ny().await?;

    // Ingen perioder i DB
    let gjeldene = Utc::now() + Duration::hours(1);
    let hadde_arbeid = ctx
        .pdl_oppdatering(Duration::hours(1))
        .kjoer_oppdatering(gjeldene)
        .await?;

    assert!(!hadde_arbeid, "Ingen perioder → skal returnere false");
    Ok(())
}

#[tokio::test]
async fn kjoer_oppdatering_hopper_over_periode_uten_pdl_person() -> Result<()> {
    let mut ctx = TestContext::ny().await?;
    let ident = "12345678901";
    let periode_id = Uuid::new_v4();

    skriv_gammel_periode(&ctx.pool, periode_id, ident).await;
    let _mock = ctx.stub_pdl_uten_person(ident).await;

    let gjeldene = Utc::now() + Duration::hours(2);
    let hadde_arbeid = ctx
        .pdl_oppdatering(Duration::hours(1))
        .kjoer_oppdatering(gjeldene)
        .await?;

    assert!(
        !hadde_arbeid,
        "PDL uten person → ingen oppdatering, returnerer false"
    );

    // sist_oppdatert skal ikke ha endret seg
    let (sist_oppdatert, trenger_kontroll) = ctx
        .les_rad(periode_id)
        .await
        .expect("Perioden skal finnes");
    assert!(
        !trenger_kontroll,
        "trenger_kontroll skal ikke ha endret seg"
    );
    assert!(
        sist_oppdatert < Utc::now().naive_utc(),
        "sist_oppdatert skal fremdeles være gammelt"
    );

    Ok(())
}

#[tokio::test]
async fn kjoer_oppdatering_haandterer_flere_perioder_i_batch() -> Result<()> {
    let mut ctx = TestContext::ny().await?;
    let identer = ["10000000001", "20000000002", "30000000003"];
    let periode_ider: Vec<Uuid> = identer.iter().map(|_| Uuid::new_v4()).collect();

    for (id, ident) in periode_ider.iter().zip(identer.iter()) {
        skriv_gammel_periode(&ctx.pool, *id, ident).await;
    }

    // Mock returnerer alle tre i én respons
    let bolk: Vec<serde_json::Value> = identer
        .iter()
        .map(|ident| {
            json!({
                "ident": ident,
                "person": {
                    "foedselsdato": [],
                    "statsborgerskap": [],
                    "opphold": [],
                    "folkeregisterpersonstatus": [],
                    "bostedsadresse": [],
                    "innflyttingTilNorge": [],
                    "utflyttingFraNorge": []
                },
                "code": "ok"
            })
        })
        .collect();
    let _mock = ctx
        .pdl_server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(json!({ "data": { "hentPersonBolk": bolk } }).to_string())
        .create_async()
        .await;

    let gjeldene = Utc::now()
        .with_nanosecond(0)
        .unwrap()
        .checked_add_signed(Duration::hours(2))
        .unwrap();
    let hadde_arbeid = ctx
        .pdl_oppdatering(Duration::hours(1))
        .kjoer_oppdatering(gjeldene)
        .await?;

    assert!(hadde_arbeid, "Skal returnere true");

    for id in &periode_ider {
        let (_, trenger_kontroll) = ctx.les_rad(*id).await.expect("Perioden skal finnes");
        assert!(trenger_kontroll, "Alle perioder skal ha trenger_kontroll=true");
    }

    Ok(())
}
