use crate::domain::hendelse_logg_status::HendelseLoggStatus::OppgaveFinnesAllerede;
use crate::domain::oppgave_type::OppgaveType;
use anyhow::Result;
use chrono::{DateTime, Utc};
use prometheus::{register_gauge_vec, GaugeVec};
use sqlx::{FromRow, Postgres, Transaction};
use std::sync::LazyLock;

static AVVERGEDE_DUPLIKATER_PER_DAG: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "veileder_oppgave_avvergede_duplikater_per_dag",
        "Antall duplikate oppgaver avverget per dag (OPPGAVE_FINNES_ALLEREDE)",
        &["dato"]
    )
    .expect("Failed to register veileder_oppgave_avvergede_duplikater_per_dag gauge")
});

pub async fn oppdater(fra_tidspunkt: DateTime<Utc>, transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let rader = hent_avvergede_duplikater_per_dag(fra_tidspunkt, transaction).await?;
    AVVERGEDE_DUPLIKATER_PER_DAG.reset();
    for rad in &rader {
        AVVERGEDE_DUPLIKATER_PER_DAG
            .with_label_values(&[&rad.dato])
            .set(rad.antall as f64);
    }
    Ok(())
}

#[derive(Debug, FromRow)]
struct AvvergedeDuplikaterPerDag {
    dato: String,
    antall: i64,
}

async fn hent_avvergede_duplikater_per_dag(
    fra_tidspunkt: DateTime<Utc>,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Vec<AvvergedeDuplikaterPerDag>> {
    let vis_siste_dager: i64 = 30;
    let rader = sqlx::query_as::<_, AvvergedeDuplikaterPerDag>(
        r#"
        SELECT TO_CHAR(DATE(ohl.tidspunkt), 'YYYY-MM-DD') AS dato, COUNT(*) AS antall
        FROM oppgave_hendelse_logg ohl
        JOIN oppgaver o ON o.id = ohl.oppgave_id
        WHERE ohl.status = $1
          AND ohl.tidspunkt >= $2
          AND o.type = $3
        GROUP BY DATE(ohl.tidspunkt)
        ORDER BY DATE(ohl.tidspunkt) DESC
        LIMIT $4
        "#,
    )
    .bind(OppgaveFinnesAllerede.to_string())
    .bind(fra_tidspunkt)
    .bind(OppgaveType::AvvistUnder18.to_string())
    .bind(vis_siste_dager)
    .fetch_all(&mut **transaction)
    .await?;

    Ok(rader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::oppgave_functions::{lagre_oppgave, oppdater_hendelse_logg};
    use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
    use crate::domain::hendelse_logg_status::HendelseLoggStatus;
    use crate::domain::oppgave::Oppgave;
    use crate::domain::oppgave_status::OppgaveStatus::Ubehandlet;
    use crate::domain::oppgave_type::OppgaveType::{AvvistUnder18, VurderOppholdsstatus};
    use anyhow::Result;
    use chrono::{Duration, TimeZone, Utc};
    use paw_test::setup_test_db::setup_test_db;
    use types::arbeidssoeker_id::ArbeidssoekerId;
    use types::identitetsnummer::Identitetsnummer;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_hent_avvergede_duplikater_per_dag() -> Result<()> {
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;
        let mut tx = pg_pool.begin().await?;

        let avvist_oppgave = Oppgave::new(AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(1), Identitetsnummer::new("12345678901".to_string()).unwrap(), Utc::now());
        let avvist_oppgave_id = lagre_oppgave(&avvist_oppgave, Uuid::new_v4(), &mut tx).await?;
        let første_dag = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
        let andre_dag = første_dag + Duration::days(1);
        let tidspunkt_før_cutoff = Utc.with_ymd_and_hms(2026, 3, 9, 0, 0, 0).unwrap();

        for tidspunkt in [første_dag, første_dag, andre_dag, tidspunkt_før_cutoff] {
            oppdater_hendelse_logg(avvist_oppgave_id, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), tidspunkt), &mut tx).await?;
        }
        oppdater_hendelse_logg(avvist_oppgave_id, HendelseLoggEntry::new(HendelseLoggStatus::OppgaveOpprettet, String::new(), første_dag), &mut tx).await?;

        let vurder_oppgave_ignorert = Oppgave::new(VurderOppholdsstatus, Ubehandlet, vec![], ArbeidssoekerId(2), Identitetsnummer::new("12345678902".to_string()).unwrap(), Utc::now());
        let vurder_oppgave_id = lagre_oppgave(&vurder_oppgave_ignorert, Uuid::new_v4(), &mut tx).await?;
        oppdater_hendelse_logg(vurder_oppgave_id, HendelseLoggEntry::new(OppgaveFinnesAllerede, String::new(), første_dag), &mut tx).await?;

        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let cutoff = Utc.with_ymd_and_hms(2026, 3, 10, 0, 0, 0).unwrap();
        let rader = hent_avvergede_duplikater_per_dag(cutoff, &mut tx).await?;

        assert_eq!(rader.len(), 2, "Skal ha to datoer etter cutoff — VurderOppholdsstatus ekskludert");
        let avvergede_duplikater_forste_dag = rader
            .iter()
            .find(|r| r.dato == "2026-03-15")
            .unwrap();
        let avvergede_duplikater_andre_dag = rader
            .iter()
            .find(|r| r.dato == "2026-03-16")
            .unwrap();
        assert_eq!(avvergede_duplikater_forste_dag.antall, 2);
        assert_eq!(avvergede_duplikater_andre_dag.antall, 1);

        Ok(())
    }
}
