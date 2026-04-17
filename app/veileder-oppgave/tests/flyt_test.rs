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
use veileder_oppgave::domain::hendelse_logg_status::HendelseLoggStatus;
use veileder_oppgave::domain::oppgave_status::OppgaveStatus;
use veileder_oppgave::domain::oppgave_type::OppgaveType;
use veileder_oppgave::hendelselogg::process_hendelselogg_message;

#[tokio::test]
async fn test_flyt_blandet_avvist_og_startet_hendelser() -> Result<()> {
    let (pg_pool, _db_container) = setup_test_db().await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;

    let mut app_config = read_application_config()?;
    app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt =
        rfc3339("2021-01-01T00:00:00Z").into();

    let etter_vannskille = rfc3339("2024-09-01T00:00:00Z");
    let foer_vannskille = rfc3339("2020-01-01T00:00:00Z");

    let a_avvist: Avvist = AvvistBuilder {
        id: 100,
        identitetsnummer: "10000000001".to_string(),
        tidspunkt: etter_vannskille,
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();
    let a_startet_vurder_opphold: Startet = StartetBuilder {
        id: 100,
        identitetsnummer: "10000000001".to_string(),
        bruker_id: "10000000001".to_string(),
        tidspunkt: etter_vannskille,
        opplysninger: HashSet::from([IkkeBosatt, ErEuEoesStatsborger]),
        ..Default::default()
    }
    .build();
    let b_startet_uten_relevante_opplysninger: Startet = StartetBuilder {
        id: 200,
        identitetsnummer: "20000000002".to_string(),
        bruker_id: "20000000002".to_string(),
        tidspunkt: etter_vannskille,
        opplysninger: HashSet::from([BosattEtterFregLoven, ErOver18Aar]),
        ..Default::default()
    }
    .build();
    let b_avvist_fra_veileder: Avvist = AvvistBuilder {
        id: 200,
        identitetsnummer: "20000000002".to_string(),
        tidspunkt: etter_vannskille,
        bruker_type: BrukerType::Veileder,
        bruker_id: "Z991459".to_string(),
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();
    let c_avvist_under_18_foer_vannskille: Avvist = AvvistBuilder {
        id: 300,
        identitetsnummer: "30000000003".to_string(),
        tidspunkt: foer_vannskille,
        opplysninger: HashSet::from([ErUnder18Aar, BosattEtterFregLoven]),
        ..Default::default()
    }
    .build();

    let meldinger = [
        lag_melding(&a_avvist.as_json(), 0),
        lag_melding(&a_startet_vurder_opphold.as_json(), 1),
        lag_melding(&a_avvist.as_json(), 2),
        lag_melding(&b_startet_uten_relevante_opplysninger.as_json(), 3),
        lag_melding(&b_avvist_fra_veileder.as_json(), 4),
        lag_melding(&c_avvist_under_18_foer_vannskille.as_json(), 5),
    ];

    for melding in &meldinger {
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(melding, &app_config, &mut tx).await?;
        tx.commit().await?;
    }

    let mut tx = pg_pool.begin().await?;

    let a_avvist_oppgave = hent_nyeste_oppgave(100, OppgaveType::AvvistUnder18, &mut tx)
        .await?
        .expect("A skal ha en AvvistUnder18-oppgave");
    assert_eq!(a_avvist_oppgave.status, OppgaveStatus::Ubehandlet);
    assert_eq!(a_avvist_oppgave.type_, OppgaveType::AvvistUnder18);
    assert_eq!(
        a_avvist_oppgave.hendelse_logg.len(),
        2,
        "A's AvvistUnder18 skal ha Opprettet + FinnesAllerede"
    );
    let statuser: Vec<_> = a_avvist_oppgave
        .hendelse_logg
        .iter()
        .map(|h| &h.status)
        .collect();
    assert!(statuser.contains(&&HendelseLoggStatus::OppgaveOpprettet));
    assert!(statuser.contains(&&HendelseLoggStatus::OppgaveFinnesAllerede));

    let a_vurder = hent_nyeste_oppgave(100, OppgaveType::VurderOpphold, &mut tx)
        .await?
        .expect("A skal ha en VurderOpphold-oppgave uavhengig av AvvistUnder18");
    assert_eq!(a_vurder.status, OppgaveStatus::Ubehandlet);
    assert_eq!(a_vurder.type_, OppgaveType::VurderOpphold);

    let b_avvist = hent_nyeste_oppgave(200, OppgaveType::AvvistUnder18, &mut tx).await?;
    assert!(b_avvist.is_none(), "B skal ikke ha AvvistUnder18-oppgave");
    let b_vurder = hent_nyeste_oppgave(200, OppgaveType::VurderOpphold, &mut tx).await?;
    assert!(b_vurder.is_none(), "B skal ikke ha VurderOpphold-oppgave");

    let c_avvist = hent_nyeste_oppgave(300, OppgaveType::AvvistUnder18, &mut tx)
        .await?
        .expect("C skal ha en Ignorert-oppgave for hendelse før vannskille");
    assert_eq!(c_avvist.status, OppgaveStatus::Ignorert);

    Ok(())
}

fn lag_melding(json: &str, offset: i64) -> OwnedMessage {
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
