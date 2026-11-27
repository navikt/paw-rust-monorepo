use paw_kafka::topics::Topic;
use sqlx::PgConnection;

use crate::features::innlesing::metadata::payload::Meldingstype;
use crate::features::innlesing::metadata::record_metadata_id::RecordMetadataId;
use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    funn::{Avvik, Vurdering},
    nivaa::Nivaa,
    oppslag::{Sett, hent_melding},
    regel::{Regel, VurderFuture},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

pub const ULIKT_IDENTITETSNUMMER: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Kritisk,
};
pub const ULIK_UTFOERT_AV: Utfall = Utfall {
    kode: 2,
    alvorlighet: Alvorlighet::Kritisk,
};
pub const ULIKT_TIDSPUNKT: Utfall = Utfall {
    kode: 3,
    alvorlighet: Alvorlighet::Kritisk,
};

/// Største tillatte avvik mellom tidspunktene. Hendelsen og perioden får
/// tidspunktet hver for seg, så de kan skille med noen millisekunder.
pub const TILLATT_TIDSAVVIK_MS: i64 = 1_000;

pub struct PeriodeStartetLikStartet;

impl Regel for PeriodeStartetLikStartet {
    fn id(&self) -> RegelId {
        RegelId::PeriodeStartetLikStartet
    }

    fn versjon(&self) -> Regelversjon {
        Regelversjon::new(2)
    }

    fn nivaa(&self) -> Nivaa {
        Nivaa::Periode
    }

    fn topics(&self) -> &'static [Topic] {
        &[Topic::Hendelselogg, Topic::Periode]
    }

    fn vurder<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rad: &'a BatchRad,
        sett: &'a Sett<'a>,
    ) -> VurderFuture<'a> {
        Box::pin(async move {
            let melding = hent_melding(conn, rad.record_metadata_id).await?;
            let Some(periode_id) = melding.periode_id else {
                return Ok(Vurdering::IkkeRelevant);
            };
            let (motpart_topic, motpart_type) = match melding.meldingstype {
                Some(Meldingstype::HendelseStartet) => {
                    (Topic::Periode, Meldingstype::PeriodeStartet)
                }
                Some(Meldingstype::PeriodeStartet) => {
                    (Topic::Hendelselogg, Meldingstype::HendelseStartet)
                }
                _ => return Ok(Vurdering::IkkeRelevant),
            };
            let motpart_id = sett.topic_id(&motpart_topic);
            let rad_motpart: Option<(RecordMetadataId, bool, bool, Option<i64>)> = sqlx::query_as(
                "SELECT m.id,
                        naa.identitetsnummer IS DISTINCT FROM m.identitetsnummer,
                        (naa.utfoert_av_type, naa.utfoert_av_id)
                            IS DISTINCT FROM (m.utfoert_av_type, m.utfoert_av_id),
                        abs(naa.utfoert_tidspunkt_ms - m.utfoert_tidspunkt_ms)
                 FROM record_metadata naa, record_metadata m
                 WHERE naa.id = $1
                   AND m.periode_id = $2 AND m.meldingstype = $3
                   AND m.topic_id = $4 AND m.record_partition = naa.record_partition
                   AND m.record_offset <= $5
                 ORDER BY m.record_offset
                 LIMIT 1",
            )
            .bind(rad.record_metadata_id)
            .bind(periode_id)
            .bind(motpart_type)
            .bind(motpart_id)
            .bind(sett.siste_offset(motpart_id))
            .fetch_optional(conn)
            .await?;
            let Some((motpart, ulik_ident, ulik_utfoert_av, tidsavvik_ms)) = rad_motpart else {
                return Ok(Vurdering::IkkeRelevant);
            };
            // Tidspunktet mangler bare sammen med utført av (CHECK i record_metadata),
            // så manglende tidsavvik fanges av ulik utført av over.
            let utfall = if ulik_ident {
                ULIKT_IDENTITETSNUMMER
            } else if ulik_utfoert_av {
                ULIK_UTFOERT_AV
            } else if tidsavvik_ms.is_some_and(|ms| ms > TILLATT_TIDSAVVIK_MS) {
                ULIKT_TIDSPUNKT
            } else {
                return Ok(Vurdering::Ok);
            };
            let mut avvik = Avvik::ny(utfall).med_melding(motpart);
            if let Some(ms) = tidsavvik_ms {
                avvik = avvik.med_detalj("tidsavvik_ms", ms);
            }
            Ok(Vurdering::Feil(avvik))
        })
    }
}
