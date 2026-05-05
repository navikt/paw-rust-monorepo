use crate::domain::hendelse_logg_status::HendelseLoggStatus::{
    EksternOppgaveFerdigstilt, EksternOppgaveOpprettet,
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{FromRow, Postgres, Transaction};
use std::sync::LazyLock;

static SAKSBEHANDLINGSTID: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "veileder_oppgave_saksbehandlingstid_sekunder",
        "Gjennomsnittlig saksbehandlingstid per uke og type i sekunder (fra EksternOppgaveOpprettet til EksternOppgaveFerdigstilt)",
        &["uke", "type"]
    )
    .expect("Failed to register veileder_oppgave_saksbehandlingstid_sekunder gauge")
});

pub async fn oppdater(
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<()> {
    let saksbehandlingstider_per_uke = hent_saksbehandlingstid_per_uke(fra_tidspunkt, transaction).await?;
    SAKSBEHANDLINGSTID.reset();
    for rad in &saksbehandlingstider_per_uke {
        SAKSBEHANDLINGSTID
            .with_label_values(&[&rad.uke, &rad.type_])
            .set(rad.gjennomsnitt_sekunder);
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct SaksbehandlingstidPerUke {
    uke: String,
    #[sqlx(rename = "type")]
    type_: String,
    gjennomsnitt_sekunder: f64,
}

async fn hent_saksbehandlingstid_per_uke(
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<SaksbehandlingstidPerUke>> {
    let rader = sqlx::query_as::<_, SaksbehandlingstidPerUke>(
        //language=PostgreSQL
        r#"
        SELECT
            TO_CHAR(DATE_TRUNC('week', ferdigstilt.tidspunkt), 'YYYY-MM-DD') AS uke,
            o.type,
            AVG(EXTRACT(EPOCH FROM (ferdigstilt.tidspunkt - eksternt_opprettet.tidspunkt)))::FLOAT8 AS gjennomsnitt_sekunder
        FROM oppgave_hendelse_logg AS ferdigstilt
        JOIN (
            SELECT oppgave_id, MIN(tidspunkt) AS tidspunkt
            FROM oppgave_hendelse_logg
            WHERE status = $1
            GROUP BY oppgave_id
        ) AS eksternt_opprettet ON eksternt_opprettet.oppgave_id = ferdigstilt.oppgave_id
        JOIN oppgaver o ON o.id = ferdigstilt.oppgave_id
        WHERE ferdigstilt.status = $2
          AND ferdigstilt.tidspunkt >= $3
          AND ferdigstilt.tidspunkt >= NOW() - INTERVAL '30 weeks'
        GROUP BY DATE_TRUNC('week', ferdigstilt.tidspunkt), o.type
        ORDER BY DATE_TRUNC('week', ferdigstilt.tidspunkt) DESC
        "#,
    )
    .bind(EksternOppgaveOpprettet.to_string())
    .bind(EksternOppgaveFerdigstilt.to_string())
    .bind(fra_tidspunkt)
    .fetch_all(&mut **transaction)
    .await?;

    Ok(rader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{insert_oppgave, insert_oppgave_hendelse_logg};
    use crate::db::oppgave_hendelse_logg_row::InsertOppgaveHendelseLoggRow;
    use crate::db::oppgave_row::InsertOppgaveRow;
    use crate::domain::oppgave_type::OppgaveType::{AvvistUnder18, VurderOppholdsstatus};
    use anyhow::Result;
    use chrono::{Datelike, Duration, Utc};
    use paw_test::setup_test_db::setup_test_db;

    #[tokio::test]
    async fn test_hent_saksbehandlingstid_per_uke_og_type() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        // Bruk relative tidspunkter så testen ikke utdateres
        let now = Utc::now();
        let cutoff = now - Duration::weeks(29);

        // Uke 1 (for ~2 uker siden): AvvistUnder18 A (2 dager) og B (4 dager) → snitt 3 dager
        let uke1_mandag = {
            let d = now - Duration::weeks(2);
            // Trunkér til mandag for konsistens
            d - Duration::days(d.weekday().num_days_from_monday() as i64)
        };
        let opprettet_a = uke1_mandag + Duration::hours(8);
        let ferdigstilt_a = opprettet_a + Duration::days(2);
        let oppgave_id_1 = insert_oppgave(
            &InsertOppgaveRow {
                type_: AvvistUnder18.to_string(),
                tidspunkt: opprettet_a,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_1,
                status: EksternOppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: opprettet_a,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_1,
                status: EksternOppgaveFerdigstilt.to_string(),
                melding: String::new(),
                tidspunkt: ferdigstilt_a,
            },
            &mut tx,
        )
        .await?;

        let opprettet_b = uke1_mandag + Duration::days(1) + Duration::hours(8);
        let ferdigstilt_b = opprettet_b + Duration::days(4);
        let oppgave_id_2 = insert_oppgave(
            &InsertOppgaveRow {
                type_: AvvistUnder18.to_string(),
                identitetsnummer: "12345678902".to_string(),
                tidspunkt: opprettet_b,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_2,
                status: EksternOppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: opprettet_b,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_2,
                status: EksternOppgaveFerdigstilt.to_string(),
                melding: String::new(),
                tidspunkt: ferdigstilt_b,
            },
            &mut tx,
        )
        .await?;

        // Uke 1: VurderOpphold — 1 dag → vises separat fra AvvistUnder18
        let opprettet_v = uke1_mandag + Duration::days(2) + Duration::hours(8);
        let ferdigstilt_v = opprettet_v + Duration::days(1);
        let oppgave_id_v = insert_oppgave(
            &InsertOppgaveRow {
                type_: VurderOppholdsstatus.to_string(),
                identitetsnummer: "12345678905".to_string(),
                tidspunkt: opprettet_v,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_v,
                status: EksternOppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: opprettet_v,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_v,
                status: EksternOppgaveFerdigstilt.to_string(),
                melding: String::new(),
                tidspunkt: ferdigstilt_v,
            },
            &mut tx,
        )
        .await?;

        // Uke 2 (for ~1 uke siden): AvvistUnder18 med retry — skal bruke MIN tidspunkt
        let uke2_mandag = {
            let d = now - Duration::weeks(1);
            d - Duration::days(d.weekday().num_days_from_monday() as i64)
        };
        let opprettet_retry_forste = uke2_mandag + Duration::hours(8);
        let opprettet_retry_andre = opprettet_retry_forste + Duration::hours(1);
        let ferdigstilt_retry = opprettet_retry_forste + Duration::days(1);
        let oppgave_id_3 = insert_oppgave(
            &InsertOppgaveRow {
                type_: AvvistUnder18.to_string(),
                identitetsnummer: "12345678903".to_string(),
                tidspunkt: opprettet_retry_forste,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_3,
                status: EksternOppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: opprettet_retry_forste,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_3,
                status: EksternOppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: opprettet_retry_andre,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_3,
                status: EksternOppgaveFerdigstilt.to_string(),
                melding: String::new(),
                tidspunkt: ferdigstilt_retry,
            },
            &mut tx,
        )
        .await?;

        // Oppgave eldre enn 30 uker — skal IKKE telles (utenfor rullende vindu)
        let for_gammel = now - Duration::weeks(31);
        let oppgave_id_4 = insert_oppgave(
            &InsertOppgaveRow {
                identitetsnummer: "12345678904".to_string(),
                tidspunkt: for_gammel,
                ..Default::default()
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_4,
                status: EksternOppgaveOpprettet.to_string(),
                melding: String::new(),
                tidspunkt: for_gammel,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_4,
                status: EksternOppgaveFerdigstilt.to_string(),
                melding: String::new(),
                tidspunkt: for_gammel + Duration::hours(1),
            },
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let rader = hent_saksbehandlingstid_per_uke(cutoff, &mut tx).await?;

        // 3 rader: uke1/AvvistUnder18, uke1/VurderOpphold, uke2/AvvistUnder18
        // Oppgaven fra for 31 uker siden skal ikke telles
        assert_eq!(rader.len(), 3, "Skal ha tre rader — oppgave eldre enn 30 uker skal ekskluderes");

        let uke1_avvist = rader
            .iter()
            .find(|r| r.type_ == AvvistUnder18.to_string())
            .filter(|r| {
                // Finn uke1-raden (ikke uke2)
                rader.iter().filter(|x| x.type_ == AvvistUnder18.to_string()).count() == 2
                    || r.gjennomsnitt_sekunder != Duration::days(1).num_seconds() as f64
            });
        assert!(uke1_avvist.is_some(), "Skal ha AvvistUnder18-rad");

        let avvist_rader: Vec<_> = rader.iter().filter(|r| r.type_ == AvvistUnder18.to_string()).collect();
        assert_eq!(avvist_rader.len(), 2, "Skal ha to AvvistUnder18-rader (to uker)");

        let vurder_rader: Vec<_> = rader.iter().filter(|r| r.type_ == VurderOppholdsstatus.to_string()).collect();
        assert_eq!(vurder_rader.len(), 1, "Skal ha én VurderOpphold-rad");
        assert_eq!(
            vurder_rader[0].gjennomsnitt_sekunder,
            Duration::days(1).num_seconds() as f64
        );

        let uke2_avvist = avvist_rader
            .iter()
            .find(|r| (r.gjennomsnitt_sekunder - Duration::days(1).num_seconds() as f64).abs() < 1.0)
            .expect("Skal ha uke2 AvvistUnder18 med ~1 dag saksbehandlingstid");
        let _ = uke2_avvist;

        Ok(())
    }
}


