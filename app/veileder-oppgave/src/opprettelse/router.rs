use super::avvist_under_18::opprett_avvist_under_18_oppgave;
use super::vurder_oppholdsstatus::opprett_vurder_oppholdsstatus_oppgave;
use crate::config::ApplicationConfig;
use anyhow::Context;
use interne_hendelser::{Avvist, Startet};
use serde_json::Value;
use sqlx::{Postgres, Transaction};

pub async fn process_hendelselogg_message(
    payload: &[u8],
    app_config: &ApplicationConfig,
    tx: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<()> {
    let hendelse_json: Value = match serde_json::from_slice(payload) {
        Ok(value) => value,
        Err(_) => {
            tracing::warn!("Feilet deserialisering av JSON fra hendelselogg, hopper over");
            return Ok(());
        }
    };
    let hendelse_type = hendelse_json["hendelseType"].as_str().unwrap_or_default();

    match hendelse_type {
        interne_hendelser::AVVIST_HENDELSE_TYPE => {
            let avvist: Avvist = serde_json::from_value(hendelse_json)
                .context("Kunne ikke deserialisere avvist hendelse")?;
            opprett_avvist_under_18_oppgave(&avvist, app_config, tx).await?;
        }
        interne_hendelser::STARTET_HENDELSE_TYPE => {
            let startet: Startet = serde_json::from_value(hendelse_json)
                .context("Kunne ikke deserialisere startet hendelse")?;
            opprett_vurder_oppholdsstatus_oppgave(&startet, tx).await?;
        }
        _ => {}
    }

    Ok(())
}
