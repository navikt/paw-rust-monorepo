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
        "avvist_til_oppgave_saksbehandlingstid_sekunder",
        "Gjennomsnittlig saksbehandlingstid per uke og type i sekunder (fra EksternOppgaveOpprettet til EksternOppgaveFerdigstilt)",
        &["uke", "type"]
    )
    .expect("Failed to register avvist_til_oppgave_saksbehandlingstid_sekunder gauge")
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
    use crate::domain::oppgave_type::OppgaveType::{AvvistUnder18, VurderOpphold};
    use anyhow::Result;
    use chrono::{Duration, TimeZone, Utc};
    use paw_test::setup_test_db::setup_test_db;

    #[tokio::test]
    async fn test_hent_saksbehandlingstid_per_uke_og_type() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();

        // Uke 1 (mandag 16. mars): AvvistUnder18 A (2 dager) og B (4 dager) → snitt 3 dager
        let opprettet_a = Utc.with_ymd_and_hms(2026, 3, 16, 8, 0, 0).unwrap();
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

        let opprettet_b = Utc.with_ymd_and_hms(2026, 3, 17, 8, 0, 0).unwrap();
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

        // Uke 1 (mandag 16. mars): VurderOpphold — 1 dag → vises separat fra AvvistUnder18
        let opprettet_v = Utc.with_ymd_and_hms(2026, 3, 18, 8, 0, 0).unwrap();
        let ferdigstilt_v = opprettet_v + Duration::days(1);
        let oppgave_id_v = insert_oppgave(
            &InsertOppgaveRow {
                type_: VurderOpphold.to_string(),
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

        // Uke 2 (mandag 23. mars): AvvistUnder18 med retry — skal bruke MIN tidspunkt
        let opprettet_retry_forste = Utc.with_ymd_and_hms(2026, 3, 23, 8, 0, 0).unwrap();
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
        for tidspunkt in [opprettet_retry_forste, opprettet_retry_andre] {
            insert_oppgave_hendelse_logg(
                &InsertOppgaveHendelseLoggRow {
                    oppgave_id: oppgave_id_3,
                    status: EksternOppgaveOpprettet.to_string(),
                    melding: String::new(),
                    tidspunkt,
                },
                &mut tx,
            )
            .await?;
        }
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

        // Oppgave før cutoff — skal ikke telles
        let foer_cutoff = Utc.with_ymd_and_hms(2026, 3, 9, 8, 0, 0).unwrap();
        let oppgave_id_4 = insert_oppgave(
            &InsertOppgaveRow {
                identitetsnummer: "12345678904".to_string(),
                tidspunkt: foer_cutoff,
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
                tidspunkt: foer_cutoff,
            },
            &mut tx,
        )
        .await?;
        insert_oppgave_hendelse_logg(
            &InsertOppgaveHendelseLoggRow {
                oppgave_id: oppgave_id_4,
                status: EksternOppgaveFerdigstilt.to_string(),
                melding: String::new(),
                tidspunkt: foer_cutoff + Duration::hours(1),
            },
            &mut tx,
        )
        .await?;

        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let rader = hent_saksbehandlingstid_per_uke(cutoff, &mut tx).await?;

        // 3 rader: uke 16.mars/AvvistUnder18, uke 16.mars/VurderOpphold, uke 23.mars/AvvistUnder18
        assert_eq!(rader.len(), 3, "Skal ha tre rader (2 typer × 1 uke + 1 type × 1 uke)");

        let uke1_avvist = rader
            .iter()
            .find(|r| r.uke == "2026-03-16" && r.type_ == AvvistUnder18.to_string())
            .expect("Skal ha uke 2026-03-16 for AVVIST_UNDER_18");
        assert_eq!(
            uke1_avvist.gjennomsnitt_sekunder,
            Duration::days(3).num_seconds() as f64
        );

        let uke1_vurder = rader
            .iter()
            .find(|r| r.uke == "2026-03-16" && r.type_ == VurderOpphold.to_string())
            .expect("Skal ha uke 2026-03-16 for VURDER_OPPHOLD");
        assert_eq!(
            uke1_vurder.gjennomsnitt_sekunder,
            Duration::days(1).num_seconds() as f64
        );

        let uke2 = rader
            .iter()
            .find(|r| r.uke == "2026-03-23" && r.type_ == AvvistUnder18.to_string())
            .expect("Skal ha uke 2026-03-23 for AVVIST_UNDER_18");
        assert_eq!(
            uke2.gjennomsnitt_sekunder,
            Duration::days(1).num_seconds() as f64
        );

        Ok(())
    }
}
