use std::collections::HashMap;

use uuid::Uuid;

use crate::dao::utgang_hendelse::{Input, InternUtgangHendelse, Output};
use crate::domain::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;

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
