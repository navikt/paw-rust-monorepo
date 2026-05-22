use interne_hendelser::{Startet, vo::BrukerType};
use types::identitetsnummer::Identitetsnummer;

use crate::{
    dao::tilstand::{Stoppet, Tilstand},
    kafka::periode_deserializer::Periode,
};

pub async fn skriv_periode_data(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    id: uuid::Uuid,
    arbeidssoeker_id: Option<i64>,
    identitetsnummer: &str,
    stoppet: Option<serde_json::Value>,
    sist_oppdatert: chrono::DateTime<chrono::Utc>,
    trenger_kontroll: bool,
    siste_kontroll_tidspunkt: Option<chrono::DateTime<chrono::Utc>>,
    tilstand: Option<super::tilstand::Tilstand>,
) -> Result<(), sqlx::Error> {
    let tilstand_json = tilstand
        .map(|t| serde_json::to_value(t))
        .transpose()
        .map_err(|e| sqlx::Error::Encode(Box::new(e)))?;
    sqlx::query(
        r#"INSERT INTO perioder (id, arbeidssoeker_id, identitetsnummer, stoppet, sist_oppdatert, trenger_kontroll, siste_kontroll_tidspunkt, tilstand)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (id) DO UPDATE SET
             arbeidssoeker_id = COALESCE(EXCLUDED.arbeidssoeker_id, perioder.arbeidssoeker_id),
             stoppet = COALESCE(EXCLUDED.stoppet, perioder.stoppet),
             sist_oppdatert = EXCLUDED.sist_oppdatert,
             trenger_kontroll = EXCLUDED.trenger_kontroll,
             siste_kontroll_tidspunkt = COALESCE(EXCLUDED.siste_kontroll_tidspunkt, perioder.siste_kontroll_tidspunkt),
             tilstand = COALESCE(EXCLUDED.tilstand, perioder.tilstand)"#,
    )
    .bind(id)
    .bind(arbeidssoeker_id)
    .bind(identitetsnummer)
    .bind(&stoppet)
    .bind(sist_oppdatert.naive_utc())
    .bind(trenger_kontroll)
    .bind(siste_kontroll_tidspunkt.map(|t| t.naive_utc()))
    .bind(&tilstand_json)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn skriv_startet_periode(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    startet: Startet,
) -> Result<(), sqlx::Error> {
    let id = startet.hendelse_id;
    let arbeidssoeker_id = startet.id;
    let identitetsnummer = Identitetsnummer::new(startet.identitetsnummer.clone())
        .expect("Ugyldig identitetsnummer i Startet");
    let stoppet = None;
    let trenger_kontroll = false;
    let siste_kontroll_tidspunkt = None;
    let sist_oppdatert = startet.tidspunkt;
    let tilstand = Some(Tilstand {
        initielle: startet.opplysninger.into_iter().collect(),
        gjeldende: None,
        forrige: None,
    });
    skriv_periode_data(
        tx,
        id,
        Some(arbeidssoeker_id),
        identitetsnummer.as_ref(),
        stoppet,
        sist_oppdatert,
        trenger_kontroll,
        siste_kontroll_tidspunkt,
        tilstand,
    )
    .await
}

pub async fn skriv_periode(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    periode: Periode,
) -> Result<(), sqlx::Error> {
    let sist_oppdatert = periode
        .avsluttet
        .as_ref()
        .map(|a| a.tidspunkt)
        .unwrap_or(periode.startet.tidspunkt);
    let id = periode.id;
    let arbeidssoeker_id = None;
    let identitetsnummer = Identitetsnummer::new(periode.identitetsnummer.clone())
        .expect("Ugyldig identitetsnummer i Periode");
    let stoppet: Option<serde_json::Value> = periode
        .avsluttet
        .map(|avsluttet| Stoppet {
            tidspunkt: avsluttet.tidspunkt.clone(),
            utfoert_av: BrukerType::from(avsluttet.utfoert_av.bruker_type),
        })
        .map(|s| serde_json::to_value(s).expect("Feil ved serialisering av Stoppet"));
    let trenger_kontroll = false;
    let siste_kontroll_tidspunkt = None;
    let tilstand = None;

    skriv_periode_data(
        tx,
        id,
        arbeidssoeker_id,
        identitetsnummer.as_ref(),
        stoppet,
        sist_oppdatert,
        trenger_kontroll,
        siste_kontroll_tidspunkt,
        tilstand,
    )
    .await
}
