use paw_kafka::topics::Topic;
use sqlx::PgConnection;

use crate::features::innlesing::metadata::{payload::Meldingstype, signature::SignatureStatus};
use crate::features::regler::{
    alvorlighet::{Alvorlighet, Utfall},
    batch::BatchRad,
    funn::Vurdering,
    nivaa::Nivaa,
    oppslag::{Sett, hent_melding},
    regel::{Regel, VurderFuture},
    regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

const fn kritisk(kode: i16) -> Utfall {
    Utfall {
        kode,
        alvorlighet: Alvorlighet::Kritisk,
    }
}

const fn advarsel(kode: i16) -> Utfall {
    Utfall {
        kode,
        alvorlighet: Alvorlighet::Advarsel,
    }
}

fn utfall(status: SignatureStatus) -> Option<Utfall> {
    use SignatureStatus as S;
    match status {
        S::Valid | S::UnsignedBeforeRequired => None,
        S::MissingSignature => Some(kritisk(1)),
        S::SignatureMismatch => Some(kritisk(2)),
        S::MalformedSignature => Some(kritisk(3)),
        S::KeyRevoked => Some(kritisk(4)),
        S::UnknownKey => Some(kritisk(5)),
        S::PayloadTooLarge => Some(advarsel(6)),
        S::NoRecordTimestamp => Some(advarsel(7)),
        S::TechnicalError => Some(advarsel(8)),
    }
}

pub struct StartetErSignert;

impl Regel for StartetErSignert {
    fn id(&self) -> RegelId {
        RegelId::StartetErSignert
    }

    fn versjon(&self) -> Regelversjon {
        Regelversjon::new(1)
    }

    fn nivaa(&self) -> Nivaa {
        Nivaa::Melding
    }

    fn topics(&self) -> &'static [Topic] {
        &[Topic::Hendelselogg]
    }

    fn vurder<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rad: &'a BatchRad,
        _sett: &'a Sett<'a>,
    ) -> VurderFuture<'a> {
        Box::pin(async move {
            let melding = hent_melding(conn, rad.record_metadata_id).await?;
            if melding.meldingstype != Some(Meldingstype::HendelseStartet) {
                return Ok(Vurdering::IkkeRelevant);
            }
            Ok(match utfall(melding.signature_status) {
                None => Vurdering::Ok,
                Some(utfall) => Vurdering::Feil(utfall),
            })
        })
    }
}
