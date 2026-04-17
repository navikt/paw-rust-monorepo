use veileder_oppgave::config::read_application_config;
use veileder_oppgave::db::oppgave_functions::hent_nyeste_oppgave;
use veileder_oppgave::domain::hendelse_logg_status::HendelseLoggStatus;
use veileder_oppgave::domain::oppgave_status::OppgaveStatus;
use veileder_oppgave::domain::oppgave_type::OppgaveType;
use veileder_oppgave::hendelselogg::process_hendelselogg_message;
use anyhow::Result;
use chrono::Utc;
use paw_test::setup_test_db::setup_test_db;
use rdkafka::message::{OwnedHeaders, OwnedMessage, Timestamp};

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

#[tokio::test]
async fn test_flyt_blandet_avvist_og_startet_hendelser() -> Result<()> {
    let (pg_pool, _db_container) = setup_test_db().await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;

    let mut app_config = read_application_config()?;
    app_config.opprett_avvist_under_18_oppgaver_fra_tidspunkt =
        chrono::DateTime::parse_from_rfc3339("2021-01-01T00:00:00Z")?
            .with_timezone(&Utc)
            .into();

    let meldinger = [
        lag_melding(A_AVVIST_UNDER_18, 0),
        lag_melding(A_STARTET_VURDER_OPPHOLD, 1),
        lag_melding(A_AVVIST_UNDER_18, 2),
        lag_melding(B_STARTET_UTEN_RELEVANTE_OPPLYSNINGER, 3),
        lag_melding(B_AVVIST_FRA_VEILEDER, 4),
        lag_melding(C_AVVIST_UNDER_18_FOER_VANNSKILLE, 5),
    ];

    for melding in &meldinger {
        let mut tx = pg_pool.begin().await?;
        process_hendelselogg_message(melding, &app_config, &mut tx).await?;
        tx.commit().await?;
    }

    let mut tx = pg_pool.begin().await?;

    let a_avvist = hent_nyeste_oppgave(100, OppgaveType::AvvistUnder18, &mut tx)
        .await?
        .expect("A skal ha en AvvistUnder18-oppgave");
    assert_eq!(a_avvist.status, OppgaveStatus::Ubehandlet);
    assert_eq!(a_avvist.type_, OppgaveType::AvvistUnder18);
    assert_eq!(
        a_avvist.hendelse_logg.len(),
        2,
        "A's AvvistUnder18 skal ha Opprettet + FinnesAllerede"
    );
    let statuser: Vec<_> = a_avvist.hendelse_logg.iter().map(|h| &h.status).collect();
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

//language=JSON
const A_AVVIST_UNDER_18: &str = r#"{
    "hendelseId": "11111111-1111-1111-1111-111111111111",
    "id": 100,
    "identitetsnummer": "10000000001",
    "metadata": {
        "tidspunkt": 1725148800.000000000,
        "utfoertAv": { "type": "SYSTEM", "id": "Testsystem" },
        "kilde": "Testkilde",
        "aarsak": "Er under 18 år"
    },
    "hendelseType": "intern.v1.avvist",
    "opplysninger": ["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN"]
}"#;

//language=JSON
const A_STARTET_VURDER_OPPHOLD: &str = r#"{
    "hendelseId": "22222222-2222-2222-2222-222222222222",
    "id": 100,
    "identitetsnummer": "10000000001",
    "metadata": {
        "tidspunkt": 1725148800.000000000,
        "utfoertAv": { "type": "SLUTTBRUKER", "id": "10000000001" },
        "kilde": "Testkilde",
        "aarsak": "Test"
    },
    "hendelseType": "intern.v1.startet",
    "opplysninger": ["IKKE_BOSATT", "ER_EU_EOES_STATSBORGER"]
}"#;

//language=JSON
const B_STARTET_UTEN_RELEVANTE_OPPLYSNINGER: &str = r#"{
    "hendelseId": "33333333-3333-3333-3333-333333333333",
    "id": 200,
    "identitetsnummer": "20000000002",
    "metadata": {
        "tidspunkt": 1725148800.000000000,
        "utfoertAv": { "type": "SLUTTBRUKER", "id": "20000000002" },
        "kilde": "Testkilde",
        "aarsak": "Test"
    },
    "hendelseType": "intern.v1.startet",
    "opplysninger": ["BOSATT_ETTER_FREG_LOVEN", "ER_OVER_18_AAR"]
}"#;

//language=JSON
const B_AVVIST_FRA_VEILEDER: &str = r#"{
    "hendelseId": "44444444-4444-4444-4444-444444444444",
    "id": 200,
    "identitetsnummer": "20000000002",
    "metadata": {
        "tidspunkt": 1725148800.000000000,
        "utfoertAv": { "type": "VEILEDER", "id": "Z991459" },
        "kilde": "Testkilde",
        "aarsak": "Er under 18 år"
    },
    "hendelseType": "intern.v1.avvist",
    "opplysninger": ["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN"]
}"#;

//language=JSON
const C_AVVIST_UNDER_18_FOER_VANNSKILLE: &str = r#"{
    "hendelseId": "55555555-5555-5555-5555-555555555555",
    "id": 300,
    "identitetsnummer": "30000000003",
    "metadata": {
        "tidspunkt": 1577836800.000000000,
        "utfoertAv": { "type": "SYSTEM", "id": "Testsystem" },
        "kilde": "Testkilde",
        "aarsak": "Er under 18 år"
    },
    "hendelseType": "intern.v1.avvist",
    "opplysninger": ["ER_UNDER_18_AAR", "BOSATT_ETTER_FREG_LOVEN"]
}"#;
