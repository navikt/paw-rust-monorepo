use crate::model::dao::bekreftelse::BekreftelseRow;
use crate::model::dao::kartlegging::KartleggingRow;
use crate::model::dao::{bekreftelse, kartlegging};
use crate::model::error::LogicError;
use chrono::{DateTime, Utc};
use eksterne_hendelser::bekreftelse::bekreftelse::Bekreftelse;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(crate) async fn utled_arbeidsledighet_for_periode_med_aktiv_kartlegging<'a>(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: &'a i64,
    periode_id: &'a Uuid,
    periode_startet: &'a DateTime<Utc>,
    arbeidsledig_fra: &'a Option<DateTime<Utc>>,
    periode_gap_grense_for_ledighet: i64,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    let arbeidsledighet = match arbeidsledig_fra.clone() {
        // Bruk ledighet fra aktiv kartlegging
        Some(arbeidssledig_fra) => Some(arbeidssledig_fra),
        // Ingen ledighet fra aktiv kartlegging, så prøv å utlede fra bekreftelser
        None => {
            // Hent bekreftelser for periode-id
            let bekreftelse_rows = bekreftelse::select_by_periode_id(tx, periode_id).await?;

            let arbeidsledighet_fra_bekreftelser =
                utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, periode_startet);
            match arbeidsledighet_fra_bekreftelser {
                // Bruk ledighet fra bekreftelser
                Some(arbeidssledig_fra) => Some(arbeidssledig_fra),
                // Ingen ledighet fra bekreftelser, så prøv tidligere kartlegging
                None => {
                    utled_arbeidsledig_fra_tidligere_kartlegging(
                        tx,
                        arbeidssoeker_id,
                        periode_id,
                        periode_startet,
                        periode_gap_grense_for_ledighet,
                    )
                    .await?
                }
            }
        }
    };

    Ok(arbeidsledighet)
}

pub(crate) async fn utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging<'a>(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: &'a i64,
    periode_id: &'a Uuid,
    periode_startet: &'a DateTime<Utc>,
    periode_gap_grense_for_ledighet: i64,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    // Hent bekreftelser for periode-id
    let bekreftelse_rows = bekreftelse::select_by_periode_id(tx, periode_id).await?;

    let bekreftelser_arbeidsledig_fra =
        utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, periode_startet);

    let arbeidsledighet = match bekreftelser_arbeidsledig_fra {
        // Om det finnes bekreftelser, bruk eventuell ledighet fra de
        Some(arbeidssledig_fra) => Some(arbeidssledig_fra),
        // Ingen bekreftelser for periode-id, prøv å overføre fra tidligere kartlegging
        None => {
            utled_arbeidsledig_fra_tidligere_kartlegging(
                tx,
                arbeidssoeker_id,
                periode_id,
                periode_startet,
                periode_gap_grense_for_ledighet,
            )
            .await?
        }
    };

    Ok(arbeidsledighet)
}

pub(crate) async fn utled_arbeidsledighet_for_bekreftelse<'a>(
    tx: &mut Transaction<'_, Postgres>,
    hendelse: &Bekreftelse,
    kartlegging_row: &KartleggingRow,
    bekreftelse_rows: &Vec<BekreftelseRow>,
    periode_gap_grense_for_ledighet: i64,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    if hendelse.svar.har_jobbet_i_denne_perioden {
        // Nuller ut ledighet hvis arbeidssøker har jobbet
        Ok(None)
    } else {
        match kartlegging_row.arbeidsledig_fra {
            // Har ikke jobbet og ledighet er satt, så behold eksisterende ledighet
            Some(arbeidsledig_fra) => Ok(Some(arbeidsledig_fra)),
            // Har ikke jobbet og ledighet er ikke satt
            None => {
                let arbeidsledig_fra = utled_arbeidsledighet_fra_bekreftelse_rows(
                    &bekreftelse_rows,
                    &kartlegging_row.arbeidssoeker_fra,
                );

                match arbeidsledig_fra {
                    Some(arbeidsledig_fra) => Ok(Some(arbeidsledig_fra)),
                    // Ingen ledighet fra denne perioden ennå, prøv å overføre fra tidligere
                    // kartlegging (samme fallback som PeriodeProcessor bruker), siden
                    // bekreftelse-only replay ellers ikke ville gjenskapt denne verdien
                    None => {
                        utled_arbeidsledig_fra_tidligere_kartlegging(
                            tx,
                            &kartlegging_row.arbeidssoeker_id,
                            &kartlegging_row.periode_id,
                            &kartlegging_row.arbeidssoeker_fra,
                            periode_gap_grense_for_ledighet,
                        )
                        .await
                    }
                }
            }
        }
    }
}

fn utled_arbeidsledighet_fra_bekreftelse_rows<'a>(
    bekreftelse_rows: &'a Vec<BekreftelseRow>,
    periode_startet: &'a DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let mut arbeidsledighet = None;

    for row in bekreftelse_rows {
        if row.har_jobbet {
            arbeidsledighet = None;
        } else if arbeidsledighet.is_none() {
            arbeidsledighet = if row.gjelder_til <= *periode_startet {
                None
            } else if row.gjelder_fra < *periode_startet {
                Some(periode_startet.clone())
            } else {
                Some(row.gjelder_fra)
            }
        }
    }

    arbeidsledighet
}

async fn utled_arbeidsledig_fra_tidligere_kartlegging<'a>(
    tx: &mut Transaction<'_, Postgres>,
    arbeidssoeker_id: &'a i64,
    periode_id: &'a Uuid,
    periode_startet: &'a DateTime<Utc>,
    periode_gap_grense_for_ledighet: i64,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    let kartlegging_rows = kartlegging::select_by_arbeidssoeker_id(tx, arbeidssoeker_id).await?;

    let arbeidsledighet = utled_arbeidsledig_fra_tidligere_kartlegging_rows(
        periode_id,
        periode_startet,
        &kartlegging_rows,
        periode_gap_grense_for_ledighet,
    )?;

    Ok(arbeidsledighet)
}

fn utled_arbeidsledig_fra_tidligere_kartlegging_rows<'a>(
    periode_id: &'a Uuid,
    periode_startet: &'a DateTime<Utc>,
    kartlegging_rows: &'a Vec<KartleggingRow>,
    periode_gap_grense_for_ledighet: i64,
) -> anyhow::Result<Option<DateTime<Utc>>> {
    let kartlegging_row = kartlegging_rows
        .iter()
        .filter(|&row| row.periode_id != *periode_id)
        .max_by_key(|&row| row.arbeidssoeker_fra);

    match kartlegging_row {
        // Ingen tidligere kartlegginger
        None => Ok(None),
        Some(row) => {
            match row.arbeidsledig_fra {
                // Ingen ledighet satt for tidligere periode
                None => Ok(None),
                Some(arbeidsledig_fra) => match row.arbeidssoeker_til {
                    // Tidligere periode er fortsatt aktiv. Dette er en feil!
                    None => Err(LogicError::precondition_failed(
                        "Tidligere kartlegging er fortsatt aktiv",
                    )
                    .into()),
                    Some(arbeidssoeker_til) => {
                        let periode_gap = *periode_startet - arbeidssoeker_til;

                        // Om det er mindre enn periode_gap_grense_dager dager siden tidligere periode
                        // ble avsluttet, bruk ledighet fra den
                        if periode_gap.num_days() < periode_gap_grense_for_ledighet {
                            Ok(Some(arbeidsledig_fra))
                        } else {
                            Ok(None)
                        }
                    }
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dao::arbeidssoeker;
    use crate::model::dao::arbeidssoeker::ArbeidssoekerRow;
    use crate::model::dao::bekreftelse::BekreftelseRow;
    use crate::model::dao::kartlegging::KartleggingRow;
    use chrono::{Duration, TimeZone};
    use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use sqlx::PgPool;
    use test_data_generator::eksterne_hendelser::create_dummy_start_periode;
    use tokio::sync::OnceCell;
    use tracing_test::traced_test;

    #[traced_test]
    #[tokio::test]
    async fn test_process_messages() -> anyhow::Result<()> {
        let context = init().await?;

        test_utled_arbeidsledighet_fra_bekreftelser(context);
        test_utled_arbeidsledighet_fra_aktiv_kartlegging(context).await?;
        test_utled_arbeidsledighet_fra_tidligere_kartlegging(context).await?;

        Ok(())
    }

    fn test_utled_arbeidsledighet_fra_bekreftelser(context: &TestContext) {
        let periode_id = context.periode_id_3;
        let periode_startet = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

        let bekreftelse_row_1 = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet - Duration::days(10),
            gjelder_til: periode_startet - Duration::days(5),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let bekreftelse_row_2 = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet - Duration::days(1),
            gjelder_til: periode_startet + Duration::days(14),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let bekreftelse_row_3 = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet + Duration::days(28),
            gjelder_til: periode_startet + Duration::days(32),
            har_jobbet: true,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let mut kartlegging_rows = vec![];

        let optional_ledighet_1 =
            utled_arbeidsledighet_fra_bekreftelse_rows(&kartlegging_rows, &periode_startet);
        assert!(optional_ledighet_1.is_none());

        kartlegging_rows.push(bekreftelse_row_1);

        let optional_ledighet_2 =
            utled_arbeidsledighet_fra_bekreftelse_rows(&kartlegging_rows, &periode_startet);
        assert!(optional_ledighet_2.is_none());

        kartlegging_rows.push(bekreftelse_row_2);

        let optional_ledighet_3 =
            utled_arbeidsledighet_fra_bekreftelse_rows(&kartlegging_rows, &periode_startet);
        assert!(optional_ledighet_3.is_some());
        let ledighet_3 = optional_ledighet_3.expect("Ledighet ikke satt");
        assert_eq!(ledighet_3, periode_startet);

        kartlegging_rows.push(bekreftelse_row_3);

        let optional_ledighet_4 =
            utled_arbeidsledighet_fra_bekreftelse_rows(&kartlegging_rows, &periode_startet);
        assert!(optional_ledighet_4.is_none());
    }

    async fn test_utled_arbeidsledighet_fra_aktiv_kartlegging(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let arbeidssoeker_id = context.arbeidssoeker_id_1;
        let periode_id = context.periode_id_4;
        let identitetsnummer = context.identitetsnummer_4;
        let periode_startet = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let periode =
            create_dummy_start_periode(identitetsnummer, periode_id, Some(periode_startet));

        let bekreftelse_row = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id,
            gjelder_fra: periode_startet,
            gjelder_til: periode_startet + Duration::days(14),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let mut tx = context.start_tx().await;

        let optional_ledighet_1 = utled_arbeidsledighet_for_periode_med_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &periode_id,
            &periode_startet,
            &Some(periode_startet + Duration::days(7)),
            14,
        )
        .await?;
        assert!(optional_ledighet_1.is_some());
        assert_eq!(
            optional_ledighet_1,
            Some(periode_startet + Duration::days(7))
        );

        let optional_ledighet_2 = utled_arbeidsledighet_for_periode_med_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &periode_id,
            &periode_startet,
            &None,
            14,
        )
        .await?;
        assert!(optional_ledighet_2.is_none());

        bekreftelse::insert(&mut tx, &bekreftelse_row).await?;

        let optional_ledighet_3 = utled_arbeidsledighet_for_periode_med_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &periode_id,
            &periode_startet,
            &None,
            14,
        )
        .await?;
        assert!(optional_ledighet_3.is_some());
        let ledighet_3 = optional_ledighet_3.expect("Ledighet ikke satt");
        assert_eq!(ledighet_3, bekreftelse_row.gjelder_fra);

        tx.commit().await.expect("Kunne ikke commit transaksjon");
        Ok(())
    }

    async fn test_utled_arbeidsledighet_fra_tidligere_kartlegging(
        context: &TestContext,
    ) -> anyhow::Result<()> {
        let arbeidssoeker_id = context.arbeidssoeker_id_5;
        let aktor_id = context.aktor_id_5;
        let identitetsnummer = context.identitetsnummer_5;
        let tidligere_periode_id = context.periode_id_5_1;
        let gjeldende_periode_id = context.periode_id_5_2;
        let tidligere_periode_startet = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let tidligere_periode_avsluttet = tidligere_periode_startet + Duration::days(90);

        let arbeidssoeker_row = ArbeidssoekerRow {
            id: arbeidssoeker_id,
            aktor_id: aktor_id.to_string(),
            identitetsnummer: identitetsnummer.to_string(),
            fornavn: None,
            mellomnavn: None,
            etternavn: None,
        };

        let tidligere_kartlegging_row = KartleggingRow {
            periode_id: tidligere_periode_id,
            arbeidssoeker_id,
            arbeidssoeker_fra: tidligere_periode_startet,
            arbeidssoeker_til: None,
            arbeidsledig_fra: None,
        };

        let arbeidsledig_fra_1 = tidligere_periode_startet + Duration::days(1);
        let arbeidsledig_fra_2 = tidligere_periode_startet + Duration::days(2);
        let arbeidsledig_fra_3 = tidligere_periode_startet + Duration::days(3);

        let bekreftelse_row = BekreftelseRow {
            id: Uuid::new_v4(),
            periode_id: gjeldende_periode_id,
            gjelder_fra: tidligere_periode_avsluttet + Duration::days(1),
            gjelder_til: tidligere_periode_avsluttet + Duration::days(15),
            har_jobbet: false,
            vil_fortsette: true,
            bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            tidspunkt: Utc::now(),
        };

        let mut tx = context.start_tx().await;

        arbeidssoeker::insert(&mut tx, &arbeidssoeker_row).await?;

        // Steg 1: Ingen bekreftelser og ingen tidligere kartlegginger
        let optional_ledighet_1 = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &gjeldende_periode_id,
            &(tidligere_periode_avsluttet + Duration::days(10)),
            14,
        )
        .await?;
        assert!(optional_ledighet_1.is_none());

        // Steg 2: Har en tidligere kartlegging, men uten ledighet satt og periode er aktiv
        kartlegging::insert(&mut tx, &tidligere_kartlegging_row).await?;
        let optional_ledighet_2 = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &gjeldende_periode_id,
            &(tidligere_periode_avsluttet + Duration::days(10)),
            14,
        )
        .await?;
        assert!(optional_ledighet_2.is_none());

        // Steg 3: Setter periode avsluttet
        kartlegging::update(
            &mut tx,
            &tidligere_periode_id,
            &Some(tidligere_periode_avsluttet),
            &None,
        )
        .await?;
        let optional_ledighet_3 = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &gjeldende_periode_id,
            &(tidligere_periode_avsluttet + Duration::days(10)),
            14,
        )
        .await?;
        assert!(optional_ledighet_3.is_none());

        // Steg 4: Perioder er avluttet, men gap mellom perioder er mer enn 14 dager
        kartlegging::update(
            &mut tx,
            &tidligere_periode_id,
            &Some(tidligere_periode_avsluttet),
            &Some(arbeidsledig_fra_2),
        )
        .await?;
        let optional_ledighet_4 = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &gjeldende_periode_id,
            &(tidligere_periode_avsluttet + Duration::days(15)),
            14,
        )
        .await?;
        assert!(optional_ledighet_4.is_none());

        // Steg 5: Perioder er avluttet, og gap mellom perioder er mindre enn 14 dager
        kartlegging::update(
            &mut tx,
            &tidligere_periode_id,
            &Some(tidligere_periode_avsluttet),
            &Some(arbeidsledig_fra_3),
        )
        .await?;
        let optional_ledighet_5 = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &gjeldende_periode_id,
            &(tidligere_periode_avsluttet + Duration::days(13)),
            14,
        )
        .await?;
        assert_eq!(optional_ledighet_5, Some(arbeidsledig_fra_3));

        // Steg 6: Det finnes bekreftelse for gjeldende periode
        let gjeldende_periode_startet_6 = bekreftelse_row.gjelder_fra;
        bekreftelse::insert(&mut tx, &bekreftelse_row).await?;
        let optional_ledighet_6 = utled_arbeidsledighet_for_periode_uten_aktiv_kartlegging(
            &mut tx,
            &arbeidssoeker_id,
            &gjeldende_periode_id,
            &gjeldende_periode_startet_6,
            14,
        )
        .await?;
        assert_eq!(optional_ledighet_6, Some(gjeldende_periode_startet_6));

        tx.commit().await.expect("Kunne ikke commit transaksjon");
        Ok(())
    }

    #[test]
    fn utleder_ingen_ledighet_for_perioder_for_start() {
        let arbeidssoeker_fra = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let gjelder_fra = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let gjelder_til = Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap();
        let bekreftelse_row = dummy_bekreftelse_row(gjelder_fra, gjelder_til, false);
        let bekreftelse_rows = vec![bekreftelse_row];
        let arbeidsledighet =
            utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, &arbeidssoeker_fra);

        assert_eq!(arbeidsledighet, None);
    }

    #[test]
    fn utleder_ingen_ledighet_naar_bekreftelse_slutter_ved_periodestart() {
        let arbeidssoeker_fra = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let gjelder_fra = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let bekreftelse_row = dummy_bekreftelse_row(gjelder_fra, arbeidssoeker_fra, false);
        let bekreftelse_rows = vec![bekreftelse_row];
        let arbeidsledighet =
            utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, &arbeidssoeker_fra);

        assert_eq!(arbeidsledighet, None);
    }

    #[test]
    fn utleder_start_for_overlap() {
        let arbeidssoeker_fra = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let gjelder_fra = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let gjelder_til = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let bekreftelse_row = dummy_bekreftelse_row(gjelder_fra, gjelder_til, false);
        let bekreftelse_rows = vec![bekreftelse_row];
        let arbeidsledighet =
            utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, &arbeidssoeker_fra);

        assert_eq!(arbeidsledighet, Some(arbeidssoeker_fra));
    }

    #[test]
    fn utleder_gjelder_fra_for_perioder_etter_start() {
        let arbeidssoeker_fra = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let gjelder_fra = Utc.with_ymd_and_hms(2024, 1, 12, 0, 0, 0).unwrap();
        let gjelder_til = Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap();
        let bekreftelse_row = dummy_bekreftelse_row(gjelder_fra, gjelder_til, false);
        let bekreftelse_rows = vec![bekreftelse_row];
        let arbeidsledighet =
            utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, &arbeidssoeker_fra);

        assert_eq!(arbeidsledighet, Some(gjelder_fra));
    }

    #[test]
    fn utleder_gjelder_fra_naar_bekreftelse_starter_ved_periodestart() {
        let arbeidssoeker_fra = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let gjelder_til = arbeidssoeker_fra + Duration::days(14);
        let bekreftelse_row = dummy_bekreftelse_row(arbeidssoeker_fra, gjelder_til, false);
        let bekreftelse_rows = vec![bekreftelse_row];
        let arbeidsledighet =
            utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, &arbeidssoeker_fra);

        assert_eq!(arbeidsledighet, Some(arbeidssoeker_fra));
    }

    #[test]
    fn utleder_ledighet_fra_bekreftelser_med_reset() {
        let arbeidssoeker_fra = Utc.with_ymd_and_hms(2024, 1, 10, 0, 0, 0).unwrap();
        let bekreftelse_rows = vec![
            dummy_bekreftelse_row(
                Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap(),
                false,
            ),
            dummy_bekreftelse_row(
                Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 1, 12, 0, 0, 0).unwrap(),
                false,
            ),
            dummy_bekreftelse_row(
                Utc.with_ymd_and_hms(2024, 1, 15, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 1, 20, 0, 0, 0).unwrap(),
                true,
            ),
            dummy_bekreftelse_row(
                Utc.with_ymd_and_hms(2024, 1, 21, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2024, 1, 25, 0, 0, 0).unwrap(),
                false,
            ),
        ];

        let arbeidsledighet =
            utled_arbeidsledighet_fra_bekreftelse_rows(&bekreftelse_rows, &arbeidssoeker_fra);

        assert_eq!(
            arbeidsledighet,
            Some(Utc.with_ymd_and_hms(2024, 1, 21, 0, 0, 0).unwrap())
        );
    }

    #[test]
    fn overforer_ikke_ledighet_naar_ingen_tidligere_perioder() {
        let periode_id = Uuid::new_v4();
        let periode_startet = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let kartlegging_rows = vec![];

        let arbeidsledighet = utled_arbeidsledig_fra_tidligere_kartlegging_rows(
            &periode_id,
            &periode_startet,
            &kartlegging_rows,
            14,
        )
        .expect("Kunne ikke utlede arbeidsledighet");

        assert_eq!(arbeidsledighet, None);
    }

    #[test]
    fn overforer_ikke_ledighet_naar_tidligere_periode_mangler_ledighet() {
        let periode_id = Uuid::new_v4();
        let periode_startet = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let kartlegging_rows = vec![dummy_kartlegging_row(
            Uuid::new_v4(),
            Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
            None,
        )];

        let arbeidsledighet = utled_arbeidsledig_fra_tidligere_kartlegging_rows(
            &periode_id,
            &periode_startet,
            &kartlegging_rows,
            14,
        )
        .expect("Kunne ikke utlede arbeidsledighet");

        assert_eq!(arbeidsledighet, None);
    }

    #[test]
    fn overforer_ikke_ledighet_naar_tidligere_periode_fortsatt_aktiv() {
        let periode_id = Uuid::new_v4();
        let periode_startet = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let kartlegging_rows = vec![dummy_kartlegging_row(
            Uuid::new_v4(),
            Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap(),
            None,
            Some(Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap()),
        )];

        match utled_arbeidsledig_fra_tidligere_kartlegging_rows(
            &periode_id,
            &periode_startet,
            &kartlegging_rows,
            14,
        ) {
            Ok(_) => panic!("Skal feile"),
            Err(e) => assert_eq!(
                e.to_string(),
                "Precondition failed: Tidligere kartlegging er fortsatt aktiv"
            ),
        }
    }

    #[test]
    fn overforer_ledighet_naar_gap_er_under_grense() {
        let periode_id = Uuid::new_v4();
        let periode_startet = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let tidligere_arbeidsledig_fra = Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
        let kartlegging_rows = vec![dummy_kartlegging_row(
            Uuid::new_v4(),
            Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 1, 25, 0, 0, 0).unwrap()),
            tidligere_arbeidsledig_fra,
        )];

        let arbeidsledighet = utled_arbeidsledig_fra_tidligere_kartlegging_rows(
            &periode_id,
            &periode_startet,
            &kartlegging_rows,
            14,
        )
        .expect("Kunne ikke utlede arbeidsledighet");

        assert_eq!(arbeidsledighet, tidligere_arbeidsledig_fra);
    }

    #[test]
    fn overforer_ikke_ledighet_naar_gap_er_over_grense() {
        let periode_id = Uuid::new_v4();
        let periode_startet = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let tidligere_arbeidsledig_fra = Some(Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap());
        let kartlegging_rows = vec![dummy_kartlegging_row(
            Uuid::new_v4(),
            Utc.with_ymd_and_hms(2025, 12, 1, 0, 0, 0).unwrap(),
            Some(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()),
            tidligere_arbeidsledig_fra,
        )];

        let arbeidsledighet = utled_arbeidsledig_fra_tidligere_kartlegging_rows(
            &periode_id,
            &periode_startet,
            &kartlegging_rows,
            14,
        )
        .expect("Kunne ikke utlede arbeidsledighet");

        assert_eq!(arbeidsledighet, None);
    }

    static INIT: OnceCell<TestContext> = OnceCell::const_new();

    async fn init() -> anyhow::Result<&'static TestContext> {
        let context = INIT
            .get_or_init(|| async {
                let postgres_guard = setup_postgres_container()
                    .await
                    .expect("Failed to start Postgres container");
                println!("Migrerer databasemodell");
                sqlx::migrate!("./migrations")
                    .run(&postgres_guard.pg_pool)
                    .await
                    .expect("Failed to run migrations");

                TestContext {
                    pg_pool: postgres_guard.pg_pool,
                    arbeidssoeker_id_1: 12345,
                    arbeidssoeker_id_5: 56789,
                    aktor_id_5: "501701234500",
                    identitetsnummer_1_1: "41017012345",
                    identitetsnummer_1_2: "01017012345",
                    identitetsnummer_4: "04017012345",
                    identitetsnummer_5: "05017012345",
                    periode_id_1: Uuid::new_v4(),
                    periode_id_2: Uuid::new_v4(),
                    periode_id_3: Uuid::new_v4(),
                    periode_id_4: Uuid::new_v4(),
                    periode_id_5_1: Uuid::new_v4(),
                    periode_id_5_2: Uuid::new_v4(),
                }
            })
            .await;

        Ok(context)
    }

    struct TestContext {
        pg_pool: PgPool,
        arbeidssoeker_id_1: i64,
        arbeidssoeker_id_5: i64,
        aktor_id_5: &'static str,
        identitetsnummer_1_1: &'static str,
        identitetsnummer_1_2: &'static str,
        identitetsnummer_4: &'static str,
        identitetsnummer_5: &'static str,
        periode_id_1: Uuid,
        periode_id_2: Uuid,
        periode_id_3: Uuid,
        periode_id_4: Uuid,
        periode_id_5_1: Uuid,
        periode_id_5_2: Uuid,
    }

    impl TestContext {
        async fn start_tx(&self) -> Transaction<'_, Postgres> {
            println!(
                "Starter transaksjon (antall ledige tråder: {})",
                self.pg_pool.num_idle()
            );
            self.pg_pool
                .begin()
                .await
                .expect("Kunne ikke starte transaksjon")
        }
    }

    fn dummy_kartlegging_row(
        periode_id: Uuid,
        arbeidssoeker_fra: DateTime<Utc>,
        arbeidssoeker_til: Option<DateTime<Utc>>,
        arbeidsledig_fra: Option<DateTime<Utc>>,
    ) -> KartleggingRow {
        KartleggingRow::new(
            periode_id,
            1,
            arbeidssoeker_fra,
            arbeidssoeker_til,
            arbeidsledig_fra,
        )
    }

    fn dummy_bekreftelse_row(
        gjelder_fra: DateTime<Utc>,
        gjelder_til: DateTime<Utc>,
        har_jobbet: bool,
    ) -> BekreftelseRow {
        BekreftelseRow::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            gjelder_fra,
            gjelder_til,
            har_jobbet,
            true,
            Bekreftelsesloesning::Arbeidssoekerregisteret
                .as_ref()
                .to_string(),
            Utc::now(),
        )
    }
}
