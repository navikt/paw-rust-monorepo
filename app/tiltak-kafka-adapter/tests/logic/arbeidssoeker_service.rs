use crate::support::config::synk_config;
use crate::support::test_data::{
    TestData, finn_tiltak_synk_by_periode_id, for_timer_siden, init_pool, opprett_arbeidssoeker,
    opprett_komplett,
};
use tiltak_kafka_adapter::logic::service::arbeidssoeker::{ArbeidssoekerService, Gjenoppretting};
use tiltak_kafka_adapter::model::status::SynkStatus;
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_manglende_oppretter_venter() {
    let pool = init_pool().await;
    let config = synk_config(5);
    let test_data = TestData::from_scratch();
    opprett_arbeidssoeker(&pool, &test_data).await;
    let service = ArbeidssoekerService::new(pool.clone(), config.clone());

    assert_eq!(service.init_manglende(&Uuid::new_v4()).await.unwrap(), 1);
    assert_eq!(service.init_manglende(&Uuid::new_v4()).await.unwrap(), 0);
    let row = finn_tiltak_synk_by_periode_id(&pool, &test_data.periode_id).await;
    assert_eq!(row.status, SynkStatus::Venter);
    assert_eq!(row.antall_feil, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn feilet_gaar_til_venter_eller_avbrutt_og_hengende_aktive_frigis() {
    let pool = init_pool().await;
    let config = synk_config(5);
    let service = ArbeidssoekerService::new(pool.clone(), config.clone());

    let feilet_ni_ganger = TestData::from_antall_feil(9);
    let feilet_ti_ganger = TestData::from_antall_feil(10);
    let mut aktiv_gammel = TestData::from_status(SynkStatus::Aktiv);
    aktiv_gammel.status_endret = for_timer_siden(3);
    let aktiv_ny = TestData::from_status(SynkStatus::Aktiv);

    opprett_komplett(&pool, &feilet_ni_ganger).await;
    opprett_komplett(&pool, &feilet_ti_ganger).await;
    opprett_komplett(&pool, &aktiv_gammel).await;
    opprett_komplett(&pool, &aktiv_ny).await;

    let resultat = service.prosesser_feilede().await.unwrap();

    assert_eq!(
        resultat,
        Gjenoppretting {
            avbrutt: 1,
            til_venter: 1,
            hengende_aktive: 1
        }
    );

    let feilet_ni_ganger_row =
        finn_tiltak_synk_by_periode_id(&pool, &feilet_ni_ganger.periode_id).await;
    let feilet_ti_ganger_row =
        finn_tiltak_synk_by_periode_id(&pool, &feilet_ti_ganger.periode_id).await;
    let aktiv_gammel_row = finn_tiltak_synk_by_periode_id(&pool, &aktiv_gammel.periode_id).await;
    let aktiv_ny_row = finn_tiltak_synk_by_periode_id(&pool, &aktiv_ny.periode_id).await;

    assert_eq!(feilet_ni_ganger_row.status, SynkStatus::Venter);
    assert_eq!(feilet_ni_ganger_row.antall_feil, 9);
    assert_eq!(feilet_ti_ganger_row.status, SynkStatus::Avbrutt);
    assert_eq!(feilet_ti_ganger_row.antall_feil, 10);
    assert_eq!(aktiv_gammel_row.status, SynkStatus::Venter);
    assert_eq!(aktiv_gammel_row.antall_feil, 0);
    assert_eq!(aktiv_ny_row.status, SynkStatus::Aktiv);
    assert_eq!(aktiv_ny_row.antall_feil, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn avbrutt_og_ferdig_gaar_til_venter_etter_ventetid() {
    let pool = init_pool().await;
    let config = synk_config(5);
    let service = ArbeidssoekerService::new(pool.clone(), config.clone());

    let mut avbrutt_gammel = TestData::from_status(SynkStatus::Avbrutt);
    avbrutt_gammel.sist_kjoert = for_timer_siden(3);
    avbrutt_gammel.antall_feil = 5;
    let mut avbrutt_ny = TestData::from_status(SynkStatus::Avbrutt);
    avbrutt_ny.sist_kjoert = for_timer_siden(1);
    avbrutt_ny.antall_feil = 5;
    let mut ferdig_gammel = TestData::from_status(SynkStatus::Ferdig);
    ferdig_gammel.sist_kjoert = for_timer_siden(3);
    let mut ferdig_ny = TestData::from_status(SynkStatus::Ferdig);
    ferdig_ny.sist_kjoert = for_timer_siden(1);

    opprett_komplett(&pool, &avbrutt_gammel).await;
    opprett_komplett(&pool, &avbrutt_ny).await;
    opprett_komplett(&pool, &ferdig_gammel).await;
    opprett_komplett(&pool, &ferdig_ny).await;

    let avbrutte_count = service.prosesser_avbrutte().await.unwrap();
    let ferdige_count = service.prosesser_ferdige().await.unwrap();

    assert_eq!(avbrutte_count, 1);
    assert_eq!(ferdige_count, 1);

    let avbrutt_gammel_row =
        finn_tiltak_synk_by_periode_id(&pool, &avbrutt_gammel.periode_id).await;
    let avbrutt_ny_row = finn_tiltak_synk_by_periode_id(&pool, &avbrutt_ny.periode_id).await;
    let ferdig_gammel_row = finn_tiltak_synk_by_periode_id(&pool, &ferdig_gammel.periode_id).await;
    let ferdig_ny_row = finn_tiltak_synk_by_periode_id(&pool, &ferdig_ny.periode_id).await;

    assert_eq!(avbrutt_gammel_row.status, SynkStatus::Venter);
    assert_eq!(avbrutt_gammel_row.antall_feil, 0);
    assert_eq!(avbrutt_ny_row.status, SynkStatus::Avbrutt);
    assert_eq!(avbrutt_ny_row.antall_feil, 5);
    assert_eq!(ferdig_gammel_row.status, SynkStatus::Venter);
    assert_eq!(ferdig_gammel_row.antall_feil, 0);
    assert_eq!(ferdig_ny_row.status, SynkStatus::Ferdig);
    assert_eq!(ferdig_ny_row.antall_feil, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resultat_og_frigi_respekterer_synk_id() {
    let pool = init_pool().await;
    let config = synk_config(5);
    let service = ArbeidssoekerService::new(pool.clone(), config.clone());

    let synk_id = Uuid::new_v4();
    let mut ferdig = TestData::from_status(SynkStatus::Aktiv);
    ferdig.synk_id = synk_id;
    let mut feilet = TestData::from_status(SynkStatus::Aktiv);
    feilet.synk_id = synk_id;
    let mut frigitt = TestData::from_status(SynkStatus::Aktiv);
    frigitt.synk_id = synk_id;
    let annen = TestData::from_status(SynkStatus::Aktiv);

    opprett_komplett(&pool, &ferdig).await;
    opprett_komplett(&pool, &feilet).await;
    opprett_komplett(&pool, &frigitt).await;
    opprett_komplett(&pool, &annen).await;

    let ferdig_count = service
        .sett_ferdig(&ferdig.periode_id, &ferdig.synk_id)
        .await
        .unwrap();
    let feilet_count = service
        .sett_feilet(&feilet.periode_id, &feilet.synk_id)
        .await
        .unwrap();
    let aktive_frigitt_count = service.frigi_aktive(&synk_id).await.unwrap();

    assert_eq!(ferdig_count, 1);
    assert_eq!(feilet_count, 1);
    assert_eq!(aktive_frigitt_count, 1);

    let ferdig_row = finn_tiltak_synk_by_periode_id(&pool, &ferdig.periode_id).await;
    let feilet_row = finn_tiltak_synk_by_periode_id(&pool, &feilet.periode_id).await;
    let frigitt_row = finn_tiltak_synk_by_periode_id(&pool, &frigitt.periode_id).await;
    let annen_row = finn_tiltak_synk_by_periode_id(&pool, &annen.periode_id).await;

    assert_eq!(ferdig_row.status, SynkStatus::Ferdig);
    assert_eq!(ferdig_row.antall_feil, 0);
    assert_eq!(feilet_row.status, SynkStatus::Feilet);
    assert_eq!(feilet_row.antall_feil, 1);
    assert_eq!(frigitt_row.status, SynkStatus::Venter);
    assert_eq!(frigitt_row.antall_feil, 0);
    assert_eq!(annen_row.status, SynkStatus::Aktiv);
    assert_eq!(annen_row.antall_feil, 0);
}
