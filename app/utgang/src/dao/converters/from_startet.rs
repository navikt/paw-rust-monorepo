use interne_hendelser::vo::Opplysninger;
use interne_hendelser::Startet;
use types::arbeidssoeker_id::ArbeidssoekerId;
use types::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use types::identitetsnummer::Identitetsnummer;

use crate::{
    dao::{
        perioder::{KontrollStatusType, PeriodeRad},
        utgang_hendelse::{Input, InternUtgangHendelse},
    },
    domain::utgang_hendelse_type::UtgangHendelseType,
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
        let opplysninger = Opplysninger(value.opplysninger.clone());
        PeriodeRad {
            id: value.hendelse_id.into(),
            arbeidssoeker_id: Some(ArbeidssoekerId(value.id)),
            trenger_kontroll: false,
            stoppet: false,
            sist_oppdatert: value.metadata.tidspunkt,
            identitetsnummer: Identitetsnummer::new(value.identitetsnummer.clone())
                .unwrap_or_else(||
                    panic!("Ugyldig identitetsnummer i rad: id={}, indikerer eksterne endringer i DB, eller kodefeil.", value.id)
                ),
            initielle_opplysninger: Some(opplysninger),
            gjeldende_opplysninger: None,
            gjeldende_tidspunkt: None,
            forrige_opplysninger: None,
            forrige_tidspunkt: None,
            siste_status: KontrollStatusType::Ukjent,
        }
    }
}
