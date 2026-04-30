use interne_hendelser::Startet;

use crate::{
    dao::utgang_hendelse::{Input, InternUtgangHendelse},
    domain::{
        Opplysninger::Opplysninger, arbeidssoekerperiode_id::ArbeidssoekerperiodeId,
        utgang_hendelse_type::UtgangHendelseType,
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
