use sqlx::PgConnection;

use crate::features::regler::{
    alvorlighet::Utfall, nivaa::Nivaa, regel_katalog::RegelId, regelversjon::Regelversjon,
};

/// Utfallet av å vurdere én melding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vurdering {
    /// Meldingen sier ikke noe om regelen, for eksempel en melding uten
    /// trace-id for en regel på trace-nivå. Eksisterende funn står.
    IkkeRelevant,
    /// Subjektet er i orden ut fra det regelen har sett.
    Ok,
    /// Subjektet er i feil ut fra det regelen har sett.
    Feil(Utfall),
}

/// Sletter funn for regelversjon `$1, $2` med samme subjekt som meldingen `$3`.
macro_rules! slett_funn {
    ($samme_subjekt:literal) => {
        concat!(
            "DELETE FROM funn f
             USING record_metadata rm, record_metadata naa
             WHERE f.regel = $1 AND f.regelversjon = $2
               AND rm.id = f.record_metadata_id
               AND naa.id = $3
               AND ",
            $samme_subjekt
        )
    };
}

/// Lagrer vurderingen for subjektet til meldingen `record_metadata_id`.
///
/// Subjektet er kolonnen i `record_metadata` som `nivaa` peker på. Et tidligere
/// funn for samme regelversjon og subjekt slettes, og ved [`Vurdering::Feil`]
/// settes et nytt inn som peker på denne meldingen. Siste vurdering vinner, så
/// en senere melding kan flippe resultatet begge veier.
pub async fn lagre_vurdering(
    conn: &mut PgConnection,
    regel: RegelId,
    versjon: Regelversjon,
    nivaa: Nivaa,
    record_metadata_id: i64,
    vurdering: Vurdering,
) -> Result<(), sqlx::Error> {
    let utfall = match vurdering {
        Vurdering::IkkeRelevant => return Ok(()),
        Vurdering::Ok => None,
        Vurdering::Feil(utfall) => Some(utfall),
    };
    let slett = match nivaa {
        Nivaa::Melding => slett_funn!("f.record_metadata_id = $3"),
        Nivaa::Trace => slett_funn!("rm.trace_id = naa.trace_id"),
        Nivaa::Periode => slett_funn!("rm.periode_id = naa.periode_id"),
        Nivaa::Arbeidssoeker => slett_funn!("rm.arbeidssoeker_id = naa.arbeidssoeker_id"),
    };
    sqlx::query(slett)
        .bind(regel)
        .bind(versjon)
        .bind(record_metadata_id)
        .execute(&mut *conn)
        .await?;
    if let Some(utfall) = utfall {
        sqlx::query(
            "INSERT INTO funn (regel, regelversjon, record_metadata_id, kode, alvorlighet)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(regel)
        .bind(versjon)
        .bind(record_metadata_id)
        .bind(utfall.kode)
        .bind(utfall.alvorlighet)
        .execute(conn)
        .await?;
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct ModentFunn {
    pub regel: RegelId,
    pub regelversjon: Regelversjon,
    pub record_metadata_id: i64,
    pub kode: i16,
}

pub async fn hent_modne_funn(conn: &mut PgConnection) -> Result<Vec<ModentFunn>, sqlx::Error> {
    sqlx::query_as(
        "SELECT regel, regelversjon, record_metadata_id, kode
         FROM modne_funn
         ORDER BY record_metadata_id",
    )
    .fetch_all(conn)
    .await
}
