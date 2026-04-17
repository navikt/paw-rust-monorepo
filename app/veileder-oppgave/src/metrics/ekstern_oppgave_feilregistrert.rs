use crate::domain::hendelse_logg_status::HendelseLoggStatus::EksternOppgaveFeilregistrert;
use anyhow::Result;
use prometheus::{register_gauge, Gauge};
use sqlx::{Postgres, Transaction};
use std::sync::LazyLock;

static EKSTERN_OPPGAVE_FEILREGISTRERT: LazyLock<Gauge> = LazyLock::new(|| {
    register_gauge!(
        "veileder_oppgave_ekstern_oppgave_feilregistrert_total",
        "Totalt antall oppgaver som har blitt feilregistrert i eksternt Oppgave API"
    )
    .expect("Failed to register veileder_oppgave_ekstern_oppgave_feilregistrert_total gauge")
});

pub async fn oppdater(transaction: &mut Transaction<'_, Postgres>) -> Result<()> {
    let antall: i64 = sqlx::query_scalar(
        //language=PostgreSQL
        r#"
        SELECT COUNT(*)
        FROM oppgave_hendelse_logg
        WHERE status = $1
        "#,
    )
    .bind(EksternOppgaveFeilregistrert.to_string())
    .fetch_one(&mut **transaction)
    .await?;

    EKSTERN_OPPGAVE_FEILREGISTRERT.set(antall as f64);
    Ok(())
}
