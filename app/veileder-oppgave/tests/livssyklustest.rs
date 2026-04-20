use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use interne_hendelser::vo::BrukerType;
use interne_hendelser::vo::Opplysning::{
    BosattEtterFregLoven, ErEuEoesStatsborger, ErOver18Aar, ErUnder18Aar, IkkeBosatt,
};
use interne_hendelser::{Avvist, Startet};
use mockito::{Matcher, Server, ServerGuard};
use paw_test::hendelse_builder::{AsJson, AvvistBuilder, StartetBuilder, rfc3339};
use paw_test::setup_test_db::{TestDbGuard, setup_test_db};
use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashSet;
use std::sync::Arc;
use texas_client::response::TokenResponse;
use texas_client::token_client::M2MTokenClient;
use veileder_oppgave::client::oppgave_client::{OPPGAVER_PATH, OppgaveApiClient};
use veileder_oppgave::config::{ApplicationConfig, OppgaveClientConfig, read_application_config};
use veileder_oppgave::db::oppgave_functions::hent_nyeste_oppgave;
use veileder_oppgave::domain::hendelse_logg_entry::HendelseLoggEntry;
use veileder_oppgave::domain::hendelse_logg_status::HendelseLoggStatus;
use veileder_oppgave::domain::oppgave_status::OppgaveStatus;
use veileder_oppgave::domain::oppgave_type::OppgaveType;
use veileder_oppgave::hendelselogg::process_hendelselogg_message;
use veileder_oppgave::opprett_oppgave_task::prosesser_ubehandlede_oppgaver;
use veileder_oppgave::process_oppgavehendelse_message::oppdater_ferdigstilte_oppgaver;

const UNDER_18_ARBEIDSSOEKER_ID: i64 = 100;
const UNDER_18_IDENT: &str = "10000000001";
const OVER_18_ARBEIDSSOEKER_ID: i64 = 200;
const OVER_18_IDENT: &str = "20000000002";
const HISTORISK_UNDER_18_ARBEIDSSOEKER_ID: i64 = 300;
const HISTORISK_UNDER_18_IDENT: &str = "30000000003";
const VEILEDER_IDENT: &str = "Z991459";

const VURDER_OPPHOLD_ARBEIDSSOEKER_ID: i64 = 400;
const VURDER_OPPHOLD_IDENT: &str = "40000000004";

#[tokio::test]
async fn test_livssyklus_happy_path() -> Result<()> {
    let mut test_context = TestContext::ny().await?;
    let tidspunkt = rfc3339("2024-09-01T12:00:00Z");

    let avvist_under_18: Avvist = AvvistBuilder {
        arbeidssoeker_id: UNDER_18_ARBEIDSSOEKER_ID,
        identitetsnummer: UNDER_18_IDENT.to_string(),
        tidspunkt,
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();
    let startet_vurder_opphold: Startet = StartetBuilder {
        arbeidssoeker_id: VURDER_OPPHOLD_ARBEIDSSOEKER_ID,
        identitetsnummer: VURDER_OPPHOLD_IDENT.to_string(),
        utfoert_av_id: VURDER_OPPHOLD_IDENT.to_string(),
        tidspunkt,
        opplysninger: HashSet::from([IkkeBosatt, ErEuEoesStatsborger]),
        ..Default::default()
    }
    .build();

    let avvist_under_18_ekstern_id: i64 = 700_001;
    let vurder_opphold_ekstern_id: i64 = 700_002;
    test_context.send_hendelselogg(0, &avvist_under_18.as_json()).await?;
    test_context.send_hendelselogg(1, &startet_vurder_opphold.as_json()).await?;

    test_context.assert_oppgave_status(
        UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        OppgaveStatus::Ubehandlet,
    )
    .await?;
    test_context.assert_oppgave_status(
        VURDER_OPPHOLD_ARBEIDSSOEKER_ID,
        OppgaveType::VurderOpphold,
        OppgaveStatus::Ubehandlet,
    )
    .await?;

    test_context.stub_opprett_oppgave_201(UNDER_18_IDENT, avvist_under_18_ekstern_id).await;
    test_context.stub_opprett_oppgave_201(VURDER_OPPHOLD_IDENT, vurder_opphold_ekstern_id).await;
    test_context.kjor_opprett_oppgave_task().await?;

    test_context.assert_oppgave_status(
        UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        OppgaveStatus::Opprettet,
    )
    .await?;
    test_context.assert_oppgave_status(
        VURDER_OPPHOLD_ARBEIDSSOEKER_ID,
        OppgaveType::VurderOpphold,
        OppgaveStatus::Opprettet,
    )
    .await?;

    test_context.send_oppgavehendelse(&bygg_oppgave_ferdigstilt_json(avvist_under_18_ekstern_id))
        .await?;
    test_context.send_oppgavehendelse(&bygg_oppgave_ferdigstilt_json(vurder_opphold_ekstern_id))
        .await?;

    let forventet_logg = &[
        HendelseLoggStatus::OppgaveOpprettet,
        HendelseLoggStatus::EksternOppgaveOpprettet,
        HendelseLoggStatus::EksternOppgaveFerdigstilt,
    ];
    test_context.assert_oppgave_status(
        UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        OppgaveStatus::Ferdigbehandlet,
    )
    .await?;
    test_context.assert_hendelse_logg(
        UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        forventet_logg,
    )
    .await?;
    test_context.assert_oppgave_status(
        VURDER_OPPHOLD_ARBEIDSSOEKER_ID,
        OppgaveType::VurderOpphold,
        OppgaveStatus::Ferdigbehandlet,
    )
    .await?;
    test_context.assert_hendelse_logg(
        VURDER_OPPHOLD_ARBEIDSSOEKER_ID,
        OppgaveType::VurderOpphold,
        forventet_logg,
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_flyt_blandet_avvist_og_startet_hendelser() -> Result<()> {
    let test_context = TestContext::ny().await?;

    let etter_vannskille = rfc3339("2024-09-01T00:00:00Z");
    let foer_vannskille = rfc3339("2020-01-01T00:00:00Z");

    let under_18_avvist: Avvist = AvvistBuilder {
        arbeidssoeker_id: UNDER_18_ARBEIDSSOEKER_ID,
        identitetsnummer: UNDER_18_IDENT.to_string(),
        tidspunkt: etter_vannskille,
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();
    let under_18_startet_vurder_opphold: Startet = StartetBuilder {
        arbeidssoeker_id: UNDER_18_ARBEIDSSOEKER_ID,
        identitetsnummer: UNDER_18_IDENT.to_string(),
        utfoert_av_id: UNDER_18_IDENT.to_string(),
        tidspunkt: etter_vannskille,
        opplysninger: HashSet::from([IkkeBosatt, ErEuEoesStatsborger]),
        ..Default::default()
    }
    .build();
    let over_18_startet_uten_relevante_opplysninger: Startet = StartetBuilder {
        arbeidssoeker_id: OVER_18_ARBEIDSSOEKER_ID,
        identitetsnummer: OVER_18_IDENT.to_string(),
        utfoert_av_id: OVER_18_IDENT.to_string(),
        tidspunkt: etter_vannskille,
        opplysninger: HashSet::from([BosattEtterFregLoven, ErOver18Aar]),
        ..Default::default()
    }
    .build();
    let over_18_avvist_av_veileder: Avvist = AvvistBuilder {
        arbeidssoeker_id: OVER_18_ARBEIDSSOEKER_ID,
        identitetsnummer: OVER_18_IDENT.to_string(),
        tidspunkt: etter_vannskille,
        bruker_type: BrukerType::Veileder,
        utfoert_av_id: VEILEDER_IDENT.to_string(),
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();
    let historisk_under_18_avvist: Avvist = AvvistBuilder {
        arbeidssoeker_id: HISTORISK_UNDER_18_ARBEIDSSOEKER_ID,
        identitetsnummer: HISTORISK_UNDER_18_IDENT.to_string(),
        tidspunkt: foer_vannskille,
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();

    let meldinger = [
        (0, under_18_avvist.as_json()),
        (1, under_18_startet_vurder_opphold.as_json()),
        (2, under_18_avvist.as_json()),
        (3, over_18_startet_uten_relevante_opplysninger.as_json()),
        (4, over_18_avvist_av_veileder.as_json()),
        (5, historisk_under_18_avvist.as_json()),
    ];
    for (offset, json) in &meldinger {
        test_context.send_hendelselogg(*offset, json).await?;
    }

    test_context
        .assert_oppgave_status(
            UNDER_18_ARBEIDSSOEKER_ID,
            OppgaveType::AvvistUnder18,
            OppgaveStatus::Ubehandlet,
        )
        .await?;
    test_context
        .assert_hendelse_logg(
            UNDER_18_ARBEIDSSOEKER_ID,
            OppgaveType::AvvistUnder18,
            &[
                HendelseLoggStatus::OppgaveOpprettet,
                HendelseLoggStatus::OppgaveFinnesAllerede,
            ],
        )
        .await?;

    test_context
        .assert_oppgave_status(
            UNDER_18_ARBEIDSSOEKER_ID,
            OppgaveType::VurderOpphold,
            OppgaveStatus::Ubehandlet,
        )
        .await?;

    let mut tx = test_context.pg_pool.begin().await?;
    assert!(
        hent_nyeste_oppgave(OVER_18_ARBEIDSSOEKER_ID, OppgaveType::AvvistUnder18, &mut tx)
            .await?
            .is_none(),
        "Over 18 skal ikke ha AvvistUnder18-oppgave"
    );
    assert!(
        hent_nyeste_oppgave(OVER_18_ARBEIDSSOEKER_ID, OppgaveType::VurderOpphold, &mut tx)
            .await?
            .is_none(),
        "Over 18 skal ikke ha VurderOpphold-oppgave"
    );
    tx.commit().await?;

    test_context
        .assert_oppgave_status(
            HISTORISK_UNDER_18_ARBEIDSSOEKER_ID,
            OppgaveType::AvvistUnder18,
            OppgaveStatus::Ignorert,
        )
        .await?;

    Ok(())
}

struct TestContext {
    pg_pool: PgPool,
    _db_container: TestDbGuard,
    server: ServerGuard,
    app_config: ApplicationConfig,
    oppgave_api_client: Arc<OppgaveApiClient>,
}

impl TestContext {
    async fn ny() -> Result<Self> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        let mut app_config = read_application_config()?;
        app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt =
            rfc3339("2021-01-01T00:00:00Z").into();

        let server = Server::new_async().await;
        let oppgave_api_client = Arc::new(OppgaveApiClient::new(
            ny_test_oppgave_client_config(server.url()),
            Arc::new(MockTokenClient),
        ));

        Ok(Self {
            pg_pool,
            _db_container,
            server,
            app_config,
            oppgave_api_client,
        })
    }

    async fn send_hendelselogg(&self, offset: i64, json: &str) -> Result<()> {
        let melding = lag_melding(offset, json);
        let mut tx = self.pg_pool.begin().await?;
        process_hendelselogg_message(&melding, &self.app_config, &mut tx).await?;
        tx.commit().await?;
        Ok(())
    }

    async fn send_oppgavehendelse(&self, json: &str) -> Result<()> {
        let melding = lag_melding(0, json);
        let mut tx = self.pg_pool.begin().await?;
        oppdater_ferdigstilte_oppgaver(
            &melding,
            *self.app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt,
            &mut tx,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn stub_opprett_oppgave_201(&mut self, ident: &str, ekstern_oppgave_id: i64) {
        self.server
            .mock("POST", OPPGAVER_PATH)
            .match_body(Matcher::Regex(format!(r#""personident":"{}"#, ident)))
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(
                json!({
                    "id": ekstern_oppgave_id,
                    "tildeltEnhetsnr": "4863",
                    "oppgavetype": "JFR",
                    "tema": "KON",
                    "prioritet": "NORM",
                    "status": "OPPRETTET",
                    "aktivDato": "2026-02-16",
                    "versjon": 1,
                })
                .to_string(),
            )
            .create_async()
            .await;
    }

    async fn kjor_opprett_oppgave_task(&self) -> Result<()> {
        prosesser_ubehandlede_oppgaver(
            *self.app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt,
            *self.app_config.opprett_oppgaver_task_batch_size,
            self.oppgave_api_client.clone(),
            self.pg_pool.clone(),
        )
        .await
    }

    async fn assert_oppgave_status(
        &self,
        arbeidssoeker_id: i64,
        oppgave_type: OppgaveType,
        forventet: OppgaveStatus,
    ) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id, oppgave_type.clone(), &mut tx)
            .await?
            .unwrap_or_else(|| {
                panic!(
                    "Forventet oppgave for arbeidssøker {} av type {:?}",
                    arbeidssoeker_id, oppgave_type
                )
            });
        tx.commit().await?;
        assert_eq!(oppgave.status, forventet);
        Ok(())
    }

    async fn assert_hendelse_logg(
        &self,
        arbeidssoeker_id: i64,
        oppgave_type: OppgaveType,
        forventede_statuser: &[HendelseLoggStatus],
    ) -> Result<()> {
        let mut tx = self.pg_pool.begin().await?;
        let oppgave = hent_nyeste_oppgave(arbeidssoeker_id, oppgave_type, &mut tx)
            .await?
            .expect("Forventet oppgave");
        tx.commit().await?;
        assert_hendelse_logg_inneholder(&oppgave.hendelse_logg, forventede_statuser);
        Ok(())
    }
}

fn lag_melding(offset: i64, json: &str) -> OwnedMessage {
    OwnedMessage::new(
        Some(json.as_bytes().to_vec()),
        None,
        "test-topic".to_string(),
        Timestamp::CreateTime(Utc::now().timestamp_micros()),
        0,
        offset,
        Some(OwnedHeaders::new()),
    )
}

fn assert_hendelse_logg_inneholder(
    hendelse_logg: &[HendelseLoggEntry],
    forventede_statuser: &[HendelseLoggStatus],
) {
    let mut faktiske: Vec<HendelseLoggStatus> =
        hendelse_logg.iter().map(|h| h.status.clone()).collect();
    let mut forventede = forventede_statuser.to_vec();
    faktiske.sort_by_key(|s| s.to_string());
    forventede.sort_by_key(|s| s.to_string());
    assert_eq!(faktiske, forventede);
}

fn bygg_oppgave_ferdigstilt_json(ekstern_oppgave_id: i64) -> String {
    json!({
        "hendelse": {
            "hendelsestype": "OPPGAVE_FERDIGSTILT",
            "tidspunkt": [2024, 9, 5, 10, 0, 0, 0]
        },
        "utfortAv": {
            "navIdent": VEILEDER_IDENT,
            "enhetsnr": "2990"
        },
        "oppgave": {
            "oppgaveId": ekstern_oppgave_id,
            "versjon": 2,
            "tilordning": null,
            "kategorisering": null,
            "behandlingsperiode": null,
            "bruker": null
        }
    })
    .to_string()
}

struct MockTokenClient;

#[async_trait]
impl M2MTokenClient for MockTokenClient {
    async fn get_token(&self, _target: String) -> Result<TokenResponse> {
        Ok(TokenResponse {
            access_token: "dummy-token".to_string(),
            expires_in: 3600,
            token_type: "Bearer".to_string(),
        })
    }
}

fn ny_test_oppgave_client_config(base_url: String) -> OppgaveClientConfig {
    OppgaveClientConfig {
        base_url: base_url.into(),
        scope: "test-scope".to_string().into(),
    }
}
