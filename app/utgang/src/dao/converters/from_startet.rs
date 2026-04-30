use interne_hendelser::Startet;

use crate::{
    dao::{
        perioder::PeriodeRad,
        utgang_hendelse::{Input, InternUtgangHendelse},
    },
    domain::{
        Opplysninger::Opplysninger, arbeidssoeker_id::ArbeidssoekerId,
        arbeidssoekerperiode_id::ArbeidssoekerperiodeId, utgang_hendelse_type::UtgangHendelseType,
    },
};

impl From<Startet> for InternUtgangHendelse<Input> {
    fn from(h: Startet) -> Self {
        InternUtgangHendelse::new(
            UtgangHendelseType::Startet,
            ArbeidssoekerperiodeId::from(h.hendelse_id),
            h.metadata.tidspunkt,
            h.metadata.utfoert_av.bruker_type,
            Some(Opplysninger(h.opplysninger)),
        )
    }
}

impl From<&Startet> for PeriodeRad {
    fn from(value: &Startet) -> Self {
        PeriodeRad {
            id: value.hendelse_id.into(),
            arbeidssoeker_id: Some(ArbeidssoekerId(value.id)),
            trenger_kontroll: false,
            stoppet: false,
            sist_oppdatert: value.metadata.tidspunkt,
        }
    }
}
