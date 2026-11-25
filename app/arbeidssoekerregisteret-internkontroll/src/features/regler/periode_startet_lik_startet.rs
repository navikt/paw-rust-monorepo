use paw_kafka::topics::Topic;
use sqlx::PgConnection;

use crate::features::innlesing::metadata::payload::Meldingstype;
use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    fremdrift::Fremdrift,
    funn::Vurdering,
    nivaa::Nivaa,
    oppslag::hent_melding,
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

pub struct PeriodeStartetLikStartet;

impl Regel for PeriodeStartetLikStartet {
    fn id(&self) -> RegelId {
        RegelId::PeriodeStartetLikStartet
    }

    fn versjon(&self) -> Regelversjon {
        Regelversjon::new(1)
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
        sett: &'a [Fremdrift],
    ) -> VurderFuture<'a> {
        Box::pin(async move {
            let melding = hent_melding(conn, rad.record_metadata_id).await?;
            let Some(periode_id) = melding.periode_id else {
                return Ok(Vurdering::IkkeRelevant);
            };
            // `sett` følger rekkefølgen i `topics()`.
            let (motpart_topic, motpart_type) = match melding.meldingstype {
                Some(Meldingstype::HendelseStartet) => (&sett[1], Meldingstype::PeriodeStartet),
                Some(Meldingstype::PeriodeStartet) => (&sett[0], Meldingstype::HendelseStartet),
                _ => return Ok(Vurdering::IkkeRelevant),
            };
            let ulikt: Option<(bool, bool, bool)> = sqlx::query_as(
                "SELECT naa.identitetsnummer IS DISTINCT FROM m.identitetsnummer,
                        (naa.utfoert_av_type, naa.utfoert_av_id)
                            IS DISTINCT FROM (m.utfoert_av_type, m.utfoert_av_id),
                        naa.utfoert_tidspunkt_ms IS DISTINCT FROM m.utfoert_tidspunkt_ms
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
            .bind(motpart_topic.topic_id)
            .bind(motpart_topic.siste_offset)
            .fetch_optional(conn)
            .await?;
            Ok(match ulikt {
                None => Vurdering::IkkeRelevant,
                Some((true, _, _)) => Vurdering::Feil(ULIKT_IDENTITETSNUMMER),
                Some((_, true, _)) => Vurdering::Feil(ULIK_UTFOERT_AV),
                Some((_, _, true)) => Vurdering::Feil(ULIKT_TIDSPUNKT),
                Some(_) => Vurdering::Ok,
            })
        })
    }
}
