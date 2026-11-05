//! Referanseregel: hver melding skal ha en gyldig traceparent.

use paw_kafka::topics::Topic;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    fremdrift::Fremdrift,
    funn::Vurdering,
    nivaa::Nivaa,
    regel::{Regel, VurderFuture},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

pub const MANGLER_TRACE_ID: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Advarsel,
};

pub struct MeldingHarTraceId;

impl Regel for MeldingHarTraceId {
    fn id(&self) -> RegelId {
        RegelId::MeldingHarTraceId
    }

    fn versjon(&self) -> Regelversjon {
        Regelversjon::new(1)
    }

    fn nivaa(&self) -> Nivaa {
        Nivaa::Melding
    }

    fn topics(&self) -> &'static [Topic] {
        &[
            Topic::Periode,
            Topic::Opplysninger,
            Topic::Profilering,
            Topic::Bekreftelse,
            Topic::Hendelselogg,
            Topic::BekreftelseHendelseLogg,
            Topic::Egenvurdering,
            Topic::PaaVegneAv,
        ]
    }

    /// Ser bare på meldingen selv, så `sett` er ikke i bruk.
    fn vurder<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rad: &'a BatchRad,
        _sett: &'a [Fremdrift],
    ) -> VurderFuture<'a> {
        Box::pin(async move {
            let trace_id: Option<Uuid> =
                sqlx::query_scalar("SELECT trace_id FROM record_metadata WHERE id = $1")
                    .bind(rad.record_metadata_id)
                    .fetch_one(conn)
                    .await?;
            Ok(match trace_id {
                Some(_) => Vurdering::Ok,
                None => Vurdering::Feil(MANGLER_TRACE_ID),
            })
        })
    }
}
