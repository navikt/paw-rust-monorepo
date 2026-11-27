use crate::model::dao::tiltak_synk::StatusStatistikkRow;
use crate::model::dao::{arbeidssoeker, tiltak_synk};
use crate::model::status::SynkStatus;
use chrono::{DateTime, Utc};
use prometheus::{GaugeVec, register_gauge_vec};
use sqlx::PgPool;
use std::sync::LazyLock;

const VEDVARENDE_FEIL_GRENSE: i32 = 5;

const ALLE_STATUSER: [SynkStatus; 5] = [
    SynkStatus::Venter,
    SynkStatus::Aktiv,
    SynkStatus::Ferdig,
    SynkStatus::Feilet,
    SynkStatus::Avbrutt,
];

static DATABASE_GAUGE: LazyLock<GaugeVec> = LazyLock::new(|| {
    register_gauge_vec!(
        "paw_tiltak_kafka_adapter_database",
        "Tiltak Kafka Adapter Database",
        &["type"]
    )
    .expect("Failed to register paw_tiltak_kafka_adapter_database gauge")
});

#[derive(Debug, PartialEq)]
pub struct SynkStatistikk {
    pub antall_per_status: Vec<(SynkStatus, i64)>,
    pub eldste_synkronisering_sekunder: f64,
    pub vedvarende_feil: i64,
}

impl SynkStatistikk {
    pub fn fra_rader(rader: &[StatusStatistikkRow], naa: DateTime<Utc>) -> Self {
        let antall_per_status = ALLE_STATUSER
            .iter()
            .map(|status| {
                let antall = rader
                    .iter()
                    .find(|rad| &rad.status == status)
                    .map_or(0, |rad| rad.antall);
                (status.clone(), antall)
            })
            .collect();
        let eldste_synkronisering_sekunder = rader
            .iter()
            .map(|rad| rad.eldste_sist_kjoert)
            .min()
            .map_or(0.0, |eldste| (naa - eldste).as_seconds_f64());
        let vedvarende_feil = rader.iter().map(|rad| rad.vedvarende_feil).sum();
        Self {
            antall_per_status,
            eldste_synkronisering_sekunder,
            vedvarende_feil,
        }
    }
}

pub(crate) async fn register_database_metrics(pg_pool: &PgPool) -> anyhow::Result<()> {
    let mut tx = pg_pool.begin().await?;
    let count = arbeidssoeker::count(&mut tx).await?;
    let rader = tiltak_synk::select_status_statistikk(&mut tx, VEDVARENDE_FEIL_GRENSE).await?;
    tx.commit().await?;

    DATABASE_GAUGE
        .with_label_values(&["arbeidssoekere_total"])
        .set(count as f64);

    let statistikk = SynkStatistikk::fra_rader(&rader, Utc::now());
    for (status, antall) in &statistikk.antall_per_status {
        let label = format!("status_{}", status.as_ref().to_lowercase());
        DATABASE_GAUGE
            .with_label_values(&[label.as_str()])
            .set(*antall as f64);
    }
    DATABASE_GAUGE
        .with_label_values(&["eldste_synkronisering_sekunder"])
        .set(statistikk.eldste_synkronisering_sekunder);
    DATABASE_GAUGE
        .with_label_values(&["vedvarende_feil"])
        .set(statistikk.vedvarende_feil as f64);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rad(
        status: SynkStatus,
        antall: i64,
        vedvarende_feil: i64,
        eldste: DateTime<Utc>,
    ) -> StatusStatistikkRow {
        StatusStatistikkRow {
            status,
            antall,
            vedvarende_feil,
            eldste_sist_kjoert: eldste,
        }
    }

    #[test]
    fn tom_tabell_gir_null_for_alle_statuser() {
        let statistikk = SynkStatistikk::fra_rader(&[], Utc::now());

        assert_eq!(statistikk.antall_per_status.len(), ALLE_STATUSER.len());
        assert!(
            statistikk
                .antall_per_status
                .iter()
                .all(|(_, antall)| *antall == 0)
        );
        assert_eq!(statistikk.eldste_synkronisering_sekunder, 0.0);
        assert_eq!(statistikk.vedvarende_feil, 0);
    }

    #[test]
    fn summerer_vedvarende_feil_og_finner_eldste_synkronisering() {
        let naa = Utc::now();
        let rader = vec![
            rad(SynkStatus::Ferdig, 4, 0, naa - chrono::Duration::hours(1)),
            rad(SynkStatus::Feilet, 3, 2, naa - chrono::Duration::hours(3)),
            rad(SynkStatus::Avbrutt, 1, 1, naa - chrono::Duration::hours(2)),
        ];

        let statistikk = SynkStatistikk::fra_rader(&rader, naa);

        assert_eq!(
            statistikk.antall_per_status,
            vec![
                (SynkStatus::Venter, 0),
                (SynkStatus::Aktiv, 0),
                (SynkStatus::Ferdig, 4),
                (SynkStatus::Feilet, 3),
                (SynkStatus::Avbrutt, 1),
            ]
        );
        assert_eq!(statistikk.eldste_synkronisering_sekunder, 3.0 * 3600.0);
        assert_eq!(statistikk.vedvarende_feil, 3);
    }
}
