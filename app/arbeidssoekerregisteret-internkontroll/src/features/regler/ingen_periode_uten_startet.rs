use paw_kafka::topics::Topic;
use sqlx::PgConnection;

use crate::features::innlesing::metadata::payload::Meldingstype;
use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    funn::Vurdering,
    nivaa::Nivaa,
    oppslag::{Sett, har_sett_periode, hent_melding},
    regel::{Regel, VurderFuture},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

pub const MANGLER_STARTET: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Kritisk,
};

pub struct IngenPeriodeUtenStartet;

impl Regel for IngenPeriodeUtenStartet {
    fn id(&self) -> RegelId {
        RegelId::IngenPeriodeUtenStartet
    }

    fn versjon(&self) -> Regelversjon {
        Regelversjon::new(1)
    }

    fn nivaa(&self) -> Nivaa {
        Nivaa::Periode
    }

    fn kontekst_typer(&self) -> &'static [Meldingstype] {
        crate::features::regler::nivaa::PERIODE_KONTEKST
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
            Ok(match melding.meldingstype {
                Some(Meldingstype::HendelseStartet) => Vurdering::Ok,
                Some(Meldingstype::PeriodeStartet) => {
                    if har_sett_periode(
                        conn,
                        sett,
                        &Topic::Hendelselogg,
                        melding.record_partition,
                        periode_id,
                        &[Meldingstype::HendelseStartet],
                    )
                    .await?
                    {
                        Vurdering::Ok
                    } else {
                        Vurdering::feil(MANGLER_STARTET)
                    }
                }
                _ => Vurdering::IkkeRelevant,
            })
        })
    }
}
