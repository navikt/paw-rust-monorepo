mod common;

use anyhow::Result;
use chrono::{Duration, Timelike, Utc};
use common::PdlTestContext;
use interne_hendelser::vo::Opplysning;
use paw_test::hendelse_builder::{rfc3339, StartetBuilder};
use std::collections::HashSet;
use utgang::dao::skriv_periode::skriv_startet_hendelse;
use uuid::Uuid;

async fn skriv_gammel_periode(ctx: &PdlTestContext, id: Uuid, ident: &str) {
    let startet = StartetBuilder {
        hendelse_id: id,
        identitetsnummer: ident.to_string(),
        utfoert_av_id: ident.to_string(),
        tidspunkt: rfc3339("2024-01-01T00:00:00Z"),
        opplysninger: HashSet::from([Opplysning::ErOver18Aar]),
        ..Default::default()
    }
    .build();
    let mut tx = ctx.pool.begin().await.unwrap();
    skriv_startet_hendelse(&mut tx, startet).await.unwrap();
    tx.commit().await.unwrap();
}

#[tokio::test]
async fn kjoer_oppdatering_oppdaterer_sist_oppdatert_og_setter_trenger_kontroll() -> Result<()> {
    let mut ctx = PdlTestContext::ny().await?;
    let ident = "12345678901";
    let id = Uuid::new_v4();

    skriv_gammel_periode(&ctx, id, ident).await;
    let _mock = ctx.stub_pdl_med_person(&[ident]).await;

    let gjeldene = (Utc::now() + Duration::hours(2)).with_nanosecond(0).unwrap();
    let hadde_arbeid = ctx.pdl_oppdatering(Duration::hours(1)).kjoer_oppdatering(gjeldene).await?;

    assert!(hadde_arbeid);
    let (sist_oppdatert, trenger_kontroll) = ctx.les_rad(id).await.unwrap();
    assert!(trenger_kontroll);
    assert_eq!(sist_oppdatert, gjeldene.naive_utc());

    Ok(())
}

#[tokio::test]
async fn kjoer_oppdatering_returnerer_false_naar_ingen_utdaterte_perioder() -> Result<()> {
    let ctx = PdlTestContext::ny().await?;
    let hadde_arbeid = ctx
        .pdl_oppdatering(Duration::hours(1))
        .kjoer_oppdatering(Utc::now() + Duration::hours(1))
        .await?;
    assert!(!hadde_arbeid);
    Ok(())
}

#[tokio::test]
async fn kjoer_oppdatering_hopper_over_periode_uten_pdl_person() -> Result<()> {
    let mut ctx = PdlTestContext::ny().await?;
    let ident = "12345678901";
    let id = Uuid::new_v4();

    skriv_gammel_periode(&ctx, id, ident).await;
    let _mock = ctx.stub_pdl_uten_person(ident).await;

    let hadde_arbeid = ctx
        .pdl_oppdatering(Duration::hours(1))
        .kjoer_oppdatering(Utc::now() + Duration::hours(2))
        .await?;

    assert!(!hadde_arbeid);
    let (_, trenger_kontroll) = ctx.les_rad(id).await.unwrap();
    assert!(!trenger_kontroll, "sist_oppdatert skal ikke endres når PDL mangler person");
    Ok(())
}

#[tokio::test]
async fn kjoer_oppdatering_haandterer_batch_med_flere_perioder() -> Result<()> {
    let mut ctx = PdlTestContext::ny().await?;
    let identer = ["10000000001", "20000000002", "30000000003"];
    let ider: Vec<Uuid> = identer.iter().map(|_| Uuid::new_v4()).collect();

    for (id, ident) in ider.iter().zip(identer.iter()) {
        skriv_gammel_periode(&ctx, *id, ident).await;
    }
    let _mock = ctx.stub_pdl_med_person(&identer).await;

    let gjeldene = (Utc::now() + Duration::hours(2)).with_nanosecond(0).unwrap();
    let hadde_arbeid = ctx.pdl_oppdatering(Duration::hours(1)).kjoer_oppdatering(gjeldene).await?;

    assert!(hadde_arbeid);
    for id in &ider {
        let (_, trenger_kontroll) = ctx.les_rad(*id).await.unwrap();
        assert!(trenger_kontroll);
    }
    Ok(())
}
