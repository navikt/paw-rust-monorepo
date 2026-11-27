use crate::batch::{TestBehandler, TestPublisher};
use crate::support::clients::ClientFactory;
use crate::support::config::synk_config;
use crate::support::mocks::{
    ErrorRequest, create_mocks_from_http_status, create_mocks_from_test_data,
};
use crate::support::test_data::{
    TestData, count_tiltak_synk_by_status, finn_tiltak_synk_by_periode_id, init_pool,
    opprett_arbeidssoeker_batch, opprett_komplett, opprett_komplett_batch,
};
use crate::support::tracing::setup_test_tracing;
use sqlx::PgPool;
use std::ops::ControlFlow;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tiltak_kafka_adapter::config::SynkConfig;
use tiltak_kafka_adapter::logic::batch::tiltak_synk::{TiltakSynkJobb, for_hver_batch};
use tiltak_kafka_adapter::logic::batch::{BatchSlutt, Kjoering};
use tiltak_kafka_adapter::logic::service::arbeidssoeker::ArbeidssoekerService;
use tiltak_kafka_adapter::model::dao::tiltak_synk;
use tiltak_kafka_adapter::model::status::SynkStatus;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn behandler_alle_batcher_til_ingen_venter() {
    let pool = init_pool().await;

    opprett_komplett_batch(&pool, 25, SynkStatus::Venter).await;

    let mut batcher = Vec::new();

    let slutt = for_hver_batch(
        &pool,
        &Uuid::new_v4(),
        10,
        &mut TestBehandler(|rader: Vec<_>| {
            batcher.push(rader.len());
            ControlFlow::Continue(())
        }),
    )
    .await
    .unwrap();

    assert_eq!(slutt, BatchSlutt::IngenVentende);
    assert_eq!(batcher, vec![10, 10, 5]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn avslutter_uten_aa_vente_naar_alle_venter_er_laast_av_annen_instans() {
    let pool = init_pool().await;

    opprett_komplett_batch(&pool, 3, SynkStatus::Venter).await;

    let mut annen_instans = pool.begin().await.unwrap();
    let laaste = tiltak_synk::select_batch(&mut annen_instans, &Uuid::new_v4(), 10)
        .await
        .unwrap();
    assert_eq!(laaste.len(), 3);

    let slutt = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        for_hver_batch(
            &pool,
            &Uuid::new_v4(),
            10,
            &mut TestBehandler(|_| panic!("skal ikke behandle noe")),
        ),
    )
    .await
    .expect("Batchløkken skal ikke vente på låste rader")
    .unwrap();

    assert_eq!(slutt, BatchSlutt::IngenVentende);
    annen_instans.commit().await.unwrap();

    let aktiv_count = count_tiltak_synk_by_status(&pool, &SynkStatus::Aktiv).await;
    assert_eq!(aktiv_count, 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stopper_naar_behandling_ber_om_det() {
    let pool = init_pool().await;

    opprett_komplett_batch(&pool, 25, SynkStatus::Venter).await;

    let mut kall = 0;

    let slutt = for_hver_batch(
        &pool,
        &Uuid::new_v4(),
        10,
        &mut TestBehandler(|_| {
            kall += 1;
            ControlFlow::Break(())
        }),
    )
    .await
    .unwrap();

    assert_eq!(slutt, BatchSlutt::Stoppet);
    assert_eq!(kall, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hele_kjoeringen_fra_init_til_ferdig() {
    setup_test_tracing().expect("OTEL setup feilet");
    let pool = init_pool().await;

    let fnr_list = vec!["01017012345", "02017012345", "03017012345"];
    let ventende: Vec<TestData> = fnr_list
        .iter()
        .map(|&f| TestData::from_identitetsnummer(f))
        .collect();

    for v in &ventende {
        opprett_komplett(&pool, v).await;
    }

    let config = synk_config(10);
    let mocks = create_mocks_from_test_data(&ventende).await;

    let publisher = TestPublisher::new(None);
    let jobb = create_tiltak_synk_jobb(&pool, &config, mocks.server.url(), publisher.clone());

    let kjoering = jobb.run(Uuid::new_v4()).await;

    assert_eq!(
        kjoering,
        Kjoering {
            synkronisert: 3,
            feilet: 0,
            frigitt: 0,
            slutt: Some(BatchSlutt::IngenVentende),
        }
    );

    assert_eq!(publisher.kall.load(Ordering::SeqCst), 3);

    for v in &ventende {
        let row = finn_tiltak_synk_by_periode_id(&pool, &v.periode_id).await;
        assert_eq!(row.status, SynkStatus::Ferdig);
        assert_eq!(row.antall_feil, 0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feilstatus_fra_mulighetsrommet_gir_feilet() {
    setup_test_tracing().expect("OTEL setup feilet");
    let pool = init_pool().await;

    let responses = vec![
        ErrorRequest::new("01017012345", 401),
        ErrorRequest::new("02017012345", 403),
        ErrorRequest::new("03017012345", 500),
    ];
    let ventende: Vec<TestData> = responses
        .iter()
        .map(|e| TestData::from_identitetsnummer(e.ident.as_str()))
        .collect();

    for v in &ventende {
        opprett_komplett(&pool, v).await;
    }

    let config = synk_config(3);
    let mocks = create_mocks_from_http_status(responses).await;

    let publisher = TestPublisher::new(None);
    let jobb = create_tiltak_synk_jobb(&pool, &config, mocks.server.url(), publisher.clone());

    let kjoering = jobb.run(Uuid::new_v4()).await;

    assert_eq!(
        kjoering,
        Kjoering {
            synkronisert: 0,
            feilet: 3,
            frigitt: 0,
            slutt: Some(BatchSlutt::Stoppet),
        }
    );

    assert_eq!(publisher.kall.load(Ordering::SeqCst), 0);

    for v in &ventende {
        let row = finn_tiltak_synk_by_periode_id(&pool, &v.periode_id).await;
        assert_eq!(row.status, SynkStatus::Feilet);
        assert_eq!(row.antall_feil, 1);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slettet_periode_under_kjoering_telles_ikke() {
    let pool = init_pool().await;

    let fnr_list = vec!["01017012345", "02017012345"];
    let ventende: Vec<TestData> = fnr_list
        .iter()
        .map(|&f| TestData::from_identitetsnummer(f))
        .collect();

    for v in &ventende {
        opprett_komplett(&pool, v).await;
    }

    let config = synk_config(10);
    let mocks = create_mocks_from_test_data(&ventende).await;
    let jobb = create_tiltak_synk_jobb(
        &pool,
        &config,
        mocks.server.url(),
        TestPublisher::new(Some(pool.clone())),
    );

    let kjoering = jobb.run(Uuid::new_v4()).await;

    assert_eq!(kjoering.synkronisert, 0);
    assert_eq!(kjoering.feilet, 0);
    assert_eq!(kjoering.slutt, Some(BatchSlutt::IngenVentende));
    let rader: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tiltak_synk")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rader, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stopper_ved_maks_feil_og_frigir_resten() {
    let pool = init_pool().await;

    opprett_arbeidssoeker_batch(&pool, 15, SynkStatus::Venter).await;

    let config = synk_config(10);
    let mocks = create_mocks_from_test_data(&vec![]).await;
    let jobb =
        create_tiltak_synk_jobb(&pool, &config, mocks.server.url(), TestPublisher::new(None));

    let kjoering = jobb.run(Uuid::new_v4()).await;

    let feilet_count = count_tiltak_synk_by_status(&pool, &SynkStatus::Feilet).await;
    let venter_count = count_tiltak_synk_by_status(&pool, &SynkStatus::Venter).await;
    let aktiv_count = count_tiltak_synk_by_status(&pool, &SynkStatus::Aktiv).await;

    assert_eq!(kjoering.feilet, 10);
    assert_eq!(kjoering.frigitt, 5);
    assert_eq!(kjoering.slutt, Some(BatchSlutt::Stoppet));
    assert_eq!(feilet_count, 10);
    assert_eq!(venter_count, 5);
    assert_eq!(aktiv_count, 0);
}

fn create_tiltak_synk_jobb(
    pool: &PgPool,
    config: &SynkConfig,
    mockito_server_url: String,
    publisher: Arc<TestPublisher>,
) -> Arc<TiltakSynkJobb<TestPublisher>> {
    let client_factory = ClientFactory::new(mockito_server_url);
    let service = ArbeidssoekerService::new(pool.clone(), config.clone());
    let key_gen_client = client_factory.create_key_gen_client();
    let mulighetsrommet_client = client_factory.create_mulighetsrommet_client();
    TiltakSynkJobb::new(
        pool.clone(),
        config.clone(),
        service,
        key_gen_client,
        mulighetsrommet_client,
        publisher,
    )
}
