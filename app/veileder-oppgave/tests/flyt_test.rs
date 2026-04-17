use anyhow::Result;
use chrono::Utc;
use interne_hendelser::vo::BrukerType;
use interne_hendelser::vo::Opplysning::{
    BosattEtterFregLoven, ErEuEoesStatsborger, ErOver18Aar, ErUnder18Aar, IkkeBosatt,
};
use interne_hendelser::{Avvist, Startet};
use paw_test::hendelse_builder::{AsJson, AvvistBuilder, StartetBuilder, rfc3339};
use paw_test::setup_test_db::setup_test_db;
use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};
use std::collections::HashSet;
use veileder_oppgave::config::read_application_config;
use veileder_oppgave::db::oppgave_functions::hent_nyeste_oppgave;
use veileder_oppgave::domain::hendelse_logg_entry::HendelseLoggEntry;
use veileder_oppgave::domain::hendelse_logg_status::HendelseLoggStatus;
use veileder_oppgave::domain::oppgave_status::OppgaveStatus;
use veileder_oppgave::domain::oppgave_type::OppgaveType;
use veileder_oppgave::hendelselogg::process_hendelselogg_message;

const UNDER_18_ARBEIDSSOEKER_ID: i64 = 100;
const UNDER_18_IDENT: &str = "10000000001";
const OVER_18_ARBEIDSSOEKER_ID: i64 = 200;
const OVER_18_IDENT: &str = "20000000002";
const HISTORISK_UNDER_18_ARBEIDSSOEKER_ID: i64 = 300;
const HISTORISK_UNDER_18_IDENT: &str = "30000000003";
const VEILEDER_IDENT: &str = "Z991459";

#[tokio::test]
async fn test_flyt_blandet_avvist_og_startet_hendelser() -> Result<()> {
    let (pg_pool, _db_container) = setup_test_db().await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;

    let mut app_config = read_application_config()?;
    app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt =
        rfc3339("2021-01-01T00:00:00Z").into();

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
        lag_melding(0, &under_18_avvist.as_json()),
        lag_melding(1, &under_18_startet_vurder_opphold.as_json()),
        lag_melding(2, &under_18_avvist.as_json()),
        lag_melding(3, &over_18_startet_uten_relevante_opplysninger.as_json()),
        lag_melding(4, &over_18_avvist_av_veileder.as_json()),
        lag_melding(5, &historisk_under_18_avvist.as_json()),
    ];

    for melding in &meldinger {
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(melding, &app_config, &mut tx).await?;
        tx.commit().await?;
    }

    let mut tx = pg_pool.begin().await?;

    let under_18_avvist_oppgave = hent_nyeste_oppgave(
        UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        &mut tx,
    )
    .await?
    .expect("Under 18 skal ha en AvvistUnder18-oppgave");
    assert_eq!(under_18_avvist_oppgave.status, OppgaveStatus::Ubehandlet);
    assert_eq!(under_18_avvist_oppgave.type_, OppgaveType::AvvistUnder18);
    assert_hendelse_logg_inneholder(
        &under_18_avvist_oppgave.hendelse_logg,
        &[
            HendelseLoggStatus::OppgaveOpprettet,
            HendelseLoggStatus::OppgaveFinnesAllerede,
        ],
    );

    let under_18_vurder_opphold_oppgave = hent_nyeste_oppgave(
        UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::VurderOpphold,
        &mut tx,
    )
    .await?
    .expect("Under 18 skal ha en VurderOpphold-oppgave uavhengig av AvvistUnder18");
    assert_eq!(
        under_18_vurder_opphold_oppgave.status,
        OppgaveStatus::Ubehandlet
    );
    assert_eq!(
        under_18_vurder_opphold_oppgave.type_,
        OppgaveType::VurderOpphold
    );

    let over_18_avvist_oppgave = hent_nyeste_oppgave(
        OVER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        &mut tx,
    )
    .await?;
    assert!(
        over_18_avvist_oppgave.is_none(),
        "Over 18 skal ikke ha AvvistUnder18-oppgave"
    );
    let over_18_vurder_opphold_oppgave = hent_nyeste_oppgave(
        OVER_18_ARBEIDSSOEKER_ID,
        OppgaveType::VurderOpphold,
        &mut tx,
    )
    .await?;
    assert!(
        over_18_vurder_opphold_oppgave.is_none(),
        "Over 18 skal ikke ha VurderOpphold-oppgave"
    );

    let historisk_under_18_oppgave = hent_nyeste_oppgave(
        HISTORISK_UNDER_18_ARBEIDSSOEKER_ID,
        OppgaveType::AvvistUnder18,
        &mut tx,
    )
    .await?
    .expect("Historisk under 18 skal ha en Ignorert-oppgave for hendelse før vannskille");
    assert_eq!(historisk_under_18_oppgave.status, OppgaveStatus::Ignorert);

    Ok(())
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
