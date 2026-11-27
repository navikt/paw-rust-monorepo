use crate::features::innlesing::metadata::record_metadata_id::RecordMetadataId;
use sqlx::PgConnection;

use crate::features::regler::{
    alvorlighet::Utfall, nivaa::Nivaa, regel_katalog::RegelId, regelversjon::Regelversjon,
};

/// Utfallet av å vurdere én melding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Vurdering {
    /// Meldingen sier ikke noe om regelen, for eksempel en melding uten
    /// trace-id for en regel på trace-nivå. Eksisterende funn står.
    IkkeRelevant,
    /// Subjektet er i orden ut fra det regelen har sett.
    Ok,
    /// Subjektet er i feil ut fra det regelen har sett.
    Feil(Avvik),
}

impl Vurdering {
    /// Feil uten andre meldinger eller detaljer enn meldingen som ble vurdert.
    pub fn feil(utfall: Utfall) -> Self {
        Self::Feil(Avvik::ny(utfall))
    }
}

/// Et funn slik regelen så det, med det som trengs for å forklare det etterpå.
/// En senere vurdering kan finne andre meldinger, så dette lagres i stedet for
/// å regnes ut på nytt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Avvik {
    pub utfall: Utfall,
    /// Andre meldinger regelen brukte. Meldingen som ble vurdert, lagres alltid
    /// først og trenger ikke være med her.
    pub meldinger: Vec<RecordMetadataId>,
    /// Tall som forklarer utfallet, for eksempel `("tidsavvik_ms", 4210)`.
    /// Aldri identifikatorer som verdier: detaljene vises i klartekst.
    pub detaljer: Vec<(&'static str, i64)>,
}

impl Avvik {
    pub fn ny(utfall: Utfall) -> Self {
        Self {
            utfall,
            meldinger: Vec::new(),
            detaljer: Vec::new(),
        }
    }

    pub fn med_melding(mut self, melding: RecordMetadataId) -> Self {
        self.meldinger.push(melding);
        self
    }

    pub fn med_detalj(mut self, navn: &'static str, verdi: i64) -> Self {
        self.detaljer.push((navn, verdi));
        self
    }
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
    record_metadata_id: RecordMetadataId,
    vurdering: Vurdering,
) -> Result<(), sqlx::Error> {
    let avvik = match vurdering {
        Vurdering::IkkeRelevant => return Ok(()),
        Vurdering::Ok => None,
        Vurdering::Feil(avvik) => Some(avvik),
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
    if let Some(avvik) = avvik {
        let mut meldinger = vec![record_metadata_id];
        for melding in avvik.meldinger {
            if !meldinger.contains(&melding) {
                meldinger.push(melding);
            }
        }
        let detaljer: serde_json::Map<String, serde_json::Value> = avvik
            .detaljer
            .into_iter()
            .map(|(navn, verdi)| (navn.to_string(), verdi.into()))
            .collect();
        sqlx::query(
            "INSERT INTO funn (regel, regelversjon, record_metadata_id, kode, alvorlighet,
                               meldinger, detaljer)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(regel)
        .bind(versjon)
        .bind(record_metadata_id)
        .bind(avvik.utfall.kode)
        .bind(avvik.utfall.alvorlighet)
        .bind(meldinger)
        .bind(sqlx::types::Json(detaljer))
        .execute(conn)
        .await?;
    }
    Ok(())
}
