use paw_kafka::topics::Topic;
use sqlx::PgConnection;

use crate::features::innlesing::metadata::signature::SignatureStatus;
use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    funn::Vurdering,
    nivaa::Nivaa,
    oppslag::Sett,
    regel::{Regel, VurderFuture},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

pub const UGYLDIG_SIGNATUR_I_TRACE: Utfall = Utfall {
    kode: 1,
    alvorlighet: Alvorlighet::Kritisk,
};

pub struct HeleTraceSignert;

impl Regel for HeleTraceSignert {
    fn id(&self) -> RegelId {
        RegelId::HeleTraceSignert
    }

    fn versjon(&self) -> Regelversjon {
        Regelversjon::new(1)
    }

    fn nivaa(&self) -> Nivaa {
        Nivaa::Trace
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

    fn vurder<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rad: &'a BatchRad,
        _sett: &'a Sett<'a>,
    ) -> VurderFuture<'a> {
        Box::pin(async move {
            let (trace_id, status): (Option<uuid::Uuid>, SignatureStatus) = sqlx::query_as(
                "SELECT trace_id, signature_status FROM record_metadata WHERE id = $1",
            )
            .bind(rad.record_metadata_id)
            .fetch_one(conn)
            .await?;
            Ok(match (trace_id, status) {
                (None, _) => Vurdering::IkkeRelevant,
                (_, SignatureStatus::Valid | SignatureStatus::UnsignedBeforeRequired) => {
                    Vurdering::IkkeRelevant
                }
                _ => Vurdering::feil(UGYLDIG_SIGNATUR_I_TRACE),
            })
        })
    }
}
