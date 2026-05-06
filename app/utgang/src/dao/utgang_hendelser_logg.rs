use std::collections::HashMap;

use uuid::Uuid;

use crate::dao::utgang_hendelse::{Input, InternUtgangHendelse, Output};
use crate::domain::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use crate::domain::utgang_hendelse_type::UtgangHendelseType;

pub struct PeriodeHendelseData {
    pub metadata_mottatt: InternUtgangHendelse<Output>,
    pub siste_pdl_data_endret: Option<InternUtgangHendelse<Output>>,
}

pub async fn hent_metadata_og_siste_pdl(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
) -> Result<HashMap<ArbeidssoekerperiodeId, PeriodeHendelseData>, sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(HashMap::new());
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    let rader = sqlx::query_as::<_, InternUtgangHendelse<Output>>(
        r#"
        SELECT id, timestamp, type, periode_id, brukertype, opplysninger
        FROM utgang_hendelser_logg
        WHERE periode_id = ANY($1)
          AND type IN ('METADATA_MOTTATT', 'PDL_DATA_ENDRET')
        ORDER BY periode_id, timestamp
        "#,
    )
    .bind(uuid_liste)
    .fetch_all(&mut **tx)
    .await?;

    let mut metadata: HashMap<ArbeidssoekerperiodeId, InternUtgangHendelse<Output>> =
        HashMap::new();
    let mut siste_pdl: HashMap<ArbeidssoekerperiodeId, InternUtgangHendelse<Output>> =
        HashMap::new();

    for rad in rader {
        let periode_id = rad.periode_id().clone();
        match rad.hendelsetype() {
            UtgangHendelseType::MetadataMottatt => {
                metadata.insert(periode_id, rad);
            }
            UtgangHendelseType::PdlDataEndret => {
                siste_pdl.insert(periode_id, rad);
            }
            _ => {}
        }
    }

    let mut result = HashMap::new();
    for (periode_id, metadata_hendelse) in metadata {
        let pdl = siste_pdl.remove(&periode_id);
        result.insert(
            periode_id,
            PeriodeHendelseData {
                metadata_mottatt: metadata_hendelse,
                siste_pdl_data_endret: pdl,
            },
        );
    }
    Ok(result)
}

pub async fn hent_hendelser(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode_ider: &[ArbeidssoekerperiodeId],
) -> Result<HashMap<ArbeidssoekerperiodeId, Vec<InternUtgangHendelse<Output>>>, sqlx::Error> {
    if periode_ider.is_empty() {
        return Ok(HashMap::new());
    }
    let uuid_liste: Vec<Uuid> = periode_ider.iter().map(|id| id.0).collect();
    let rader = sqlx::query_as::<_, InternUtgangHendelse<Output>>(
        r#"
        SELECT id, timestamp, type, periode_id, brukertype, opplysninger
        FROM utgang_hendelser_logg
        WHERE periode_id = ANY($1)
        ORDER BY periode_id, timestamp
        "#,
    )
    .bind(uuid_liste)
    .fetch_all(&mut **tx)
    .await?;

    let mut result: HashMap<ArbeidssoekerperiodeId, Vec<InternUtgangHendelse<Output>>> =
        HashMap::new();
    for rad in rader {
        let periode_id = rad.periode_id().clone();
        result.entry(periode_id).or_default().push(rad);
    }
    Ok(result)
}

pub async fn skriv_hendelser(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    hendelser: Vec<InternUtgangHendelse<Input>>,
) -> Result<(), sqlx::Error> {
    if hendelser.is_empty() {
        return Ok(());
    }
    let mut builder = sqlx::QueryBuilder::new(
        r#"INSERT INTO utgang_hendelser_logg (
            timestamp,
            type,
            periode_id,
            brukertype,
            opplysninger
        ) "#,
    );
    builder.push_values(hendelser, |mut b, h| {
        let opplysninger = h.opplysninger().map(|o| o.to_string_vector());
        b.push_bind(h.timestamp())
            .push_bind(h.hendelsetype().to_string())
            .push_bind(h.periode_id().0)
            .push_bind(h.brukertype().to_string())
            .push_bind(opplysninger);
    });
    builder.build().execute(&mut **tx).await?;
    Ok(())
}
