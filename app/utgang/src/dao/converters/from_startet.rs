use interne_hendelser::Startet;

use crate::{
    dao::{
        perioder::PeriodeRad,
        utgang_hendelse::{Input, InternUtgangHendelse},
    },
    domain::{
        arbeidssoeker_id::ArbeidssoekerId, arbeidssoekerperiode_id::ArbeidssoekerperiodeId,
        identitetsnummer::Identitetsnummer, opplysninger::Opplysninger,
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

impl From<&Startet> for PeriodeRad {
    fn from(value: &Startet) -> Self {
        PeriodeRad {
            id: value.hendelse_id.into(),
            arbeidssoeker_id: Some(ArbeidssoekerId(value.id)),
            trenger_kontroll: false,
            stoppet: false,
            sist_oppdatert: value.metadata.tidspunkt,
            identitetsnummer: Identitetsnummer::new(value.identitetsnummer.clone())
                .unwrap_or_else(||
                    //Vi skriver 'Identitetsnummer'til db, dermed skal vi kunne lese de tilbake uten
                    //problemer. Hvis vi ikke klarer det, indikerer det at data i DB har blitt
                    //endret utenfor vår kontroll, eller at det er en feil i koden som skriver til
                    //DB.
                    panic!("Ugyldig identitetsnummer i rad: id={}, indikerer eksterne endringer i DB, eller kodefeil.", value.id,)
                ),
        }
    }
}
