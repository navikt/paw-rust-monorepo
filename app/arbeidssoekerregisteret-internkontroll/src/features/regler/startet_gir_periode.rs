use paw_kafka::topics::Topic;
use sqlx::PgConnection;

use crate::features::innlesing::metadata::payload::Meldingstype;
use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    funn::Vurdering,
    nivaa::Nivaa,
    oppslag::{Sett, forrige_for_arbeidssoeker, har_sett_periode, hent_melding},
    regel::{Regel, VurderFuture},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

pub const MANGLER_PERIODE: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Kritisk,
};

pub struct StartetGirPeriode;

impl Regel for StartetGirPeriode {
    fn id(&self) -> RegelId {
        RegelId::StartetGirPeriode
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
        sett: &'a Sett<'a>,
    ) -> VurderFuture<'a> {
        Box::pin(async move {
            let melding = hent_melding(conn, rad.record_metadata_id).await?;
            let Some(periode_id) = melding.periode_id else {
                return Ok(Vurdering::IkkeRelevant);
            };
            match melding.meldingstype {
                Some(Meldingstype::PeriodeStartet | Meldingstype::PeriodeAvsluttet) => {
                    Ok(Vurdering::Ok)
                }
                Some(Meldingstype::HendelseStartet) => {
                    let har_periode = har_sett_periode(
                        conn,
                        sett,
                        &Topic::Periode,
                        melding.record_partition,
                        periode_id,
                        &[],
                    )
                    .await?;
                    if har_periode {
                        return Ok(Vurdering::Ok);
                    }
                    let forrige = forrige_for_arbeidssoeker(
                        conn,
                        sett,
                        &melding,
                        &[
                            Meldingstype::HendelseStartet,
                            Meldingstype::HendelseAvsluttet,
                        ],
                    )
                    .await?;
                    Ok(match forrige {
                        Some(Meldingstype::HendelseStartet) => Vurdering::Ok,
                        _ => Vurdering::Feil(MANGLER_PERIODE),
                    })
                }
                _ => Ok(Vurdering::IkkeRelevant),
            }
        })
    }
}
