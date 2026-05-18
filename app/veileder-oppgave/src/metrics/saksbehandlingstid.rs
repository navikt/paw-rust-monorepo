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
    use crate::db::oppgave_functions::{lagre_oppgave, oppdater_hendelse_logg};
    use crate::domain::hendelse_logg_entry::HendelseLoggEntry;
    use crate::domain::oppgave::Oppgave;
    use crate::domain::oppgave_status::OppgaveStatus::Ubehandlet;
    use crate::domain::oppgave_type::OppgaveType::{AvvistUnder18, VurderOppholdsstatus};
    use anyhow::Result;
    use chrono::{Datelike, Duration, Utc};
    use paw_test::setup_test_db::setup_test_db;
    use types::arbeidssoeker_id::ArbeidssoekerId;
    use types::identitetsnummer::Identitetsnummer;
    use uuid::Uuid;

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
        let avvist_a = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(1), Identitetsnummer::new("12345678901".to_string()).unwrap(), opprettet_a);
        let oppgave_id_1 = lagre_oppgave(&avvist_a, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_1, HendelseLoggEntry::new(EksternOppgaveOpprettet, String::new(), opprettet_a), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_1, HendelseLoggEntry::new(EksternOppgaveFerdigstilt, String::new(), ferdigstilt_a), &mut tx).await?;

        let opprettet_b = uke1_mandag + Duration::days(1) + Duration::hours(8);
        let ferdigstilt_b = opprettet_b + Duration::days(4);
        let avvist_b = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(2), Identitetsnummer::new("12345678902".to_string()).unwrap(), opprettet_b);
        let oppgave_id_2 = lagre_oppgave(&avvist_b, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_2, HendelseLoggEntry::new(EksternOppgaveOpprettet, String::new(), opprettet_b), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_2, HendelseLoggEntry::new(EksternOppgaveFerdigstilt, String::new(), ferdigstilt_b), &mut tx).await?;

        // Uke 1: VurderOppholdsstatus — 1 dag → vises separat fra AvvistUnder18
        let opprettet_v = uke1_mandag + Duration::days(2) + Duration::hours(8);
        let ferdigstilt_v = opprettet_v + Duration::days(1);
        let vurder_v = Oppgave::new(Uuid::new_v4(), VurderOppholdsstatus, Ubehandlet, vec![], ArbeidssoekerId(3), Identitetsnummer::new("12345678905".to_string()).unwrap(), opprettet_v);
        let oppgave_id_v = lagre_oppgave(&vurder_v, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_v, HendelseLoggEntry::new(EksternOppgaveOpprettet, String::new(), opprettet_v), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_v, HendelseLoggEntry::new(EksternOppgaveFerdigstilt, String::new(), ferdigstilt_v), &mut tx).await?;

        // Uke 2 (for ~1 uke siden): AvvistUnder18 med retry — skal bruke MIN tidspunkt
        let uke2_mandag = {
            let d = now - Duration::weeks(1);
            d - Duration::days(d.weekday().num_days_from_monday() as i64)
        };
        let opprettet_retry_forste = uke2_mandag + Duration::hours(8);
        let opprettet_retry_andre = opprettet_retry_forste + Duration::hours(1);
        let ferdigstilt_retry = opprettet_retry_forste + Duration::days(1);
        let avvist_retry = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(4), Identitetsnummer::new("12345678903".to_string()).unwrap(), opprettet_retry_forste);
        let oppgave_id_3 = lagre_oppgave(&avvist_retry, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_3, HendelseLoggEntry::new(EksternOppgaveOpprettet, String::new(), opprettet_retry_forste), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_3, HendelseLoggEntry::new(EksternOppgaveOpprettet, String::new(), opprettet_retry_andre), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_3, HendelseLoggEntry::new(EksternOppgaveFerdigstilt, String::new(), ferdigstilt_retry), &mut tx).await?;

        // Oppgave eldre enn 30 uker — skal IKKE telles (utenfor rullende vindu)
        let for_gammel = now - Duration::weeks(31);
        let for_gammel_avvist = Oppgave::new(Uuid::new_v4(), AvvistUnder18, Ubehandlet, vec![], ArbeidssoekerId(5), Identitetsnummer::new("12345678904".to_string()).unwrap(), for_gammel);
        let oppgave_id_4 = lagre_oppgave(&for_gammel_avvist, &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_4, HendelseLoggEntry::new(EksternOppgaveOpprettet, String::new(), for_gammel), &mut tx).await?;
        oppdater_hendelse_logg(oppgave_id_4, HendelseLoggEntry::new(EksternOppgaveFerdigstilt, String::new(), for_gammel + Duration::hours(1)), &mut tx).await?;

        tx.commit().await?;

        let mut tx = pg_pool.begin().await?;
        let rader = hent_saksbehandlingstid_per_uke(cutoff, &mut tx).await?;

        // 3 rader: uke1/AvvistUnder18, uke1/VurderOppholdsstatus, uke2/AvvistUnder18
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
        assert_eq!(vurder_rader.len(), 1, "Skal ha én VurderOppholdsstatus-rad");
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


