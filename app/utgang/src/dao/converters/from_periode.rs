use interne_hendelser::vo::BrukerType;
use types::identitetsnummer::Identitetsnummer;

use crate::dao::perioder::PeriodeRad;
use crate::dao::utgang_hendelse::{Input, InternUtgangHendelse};
use crate::domain::{
    arbeidssoekerperiode_id::ArbeidssoekerperiodeId, utgang_hendelse_type::UtgangHendelseType,
};
use crate::kafka::periode_deserializer::{BrukerType as PeriodeTopicBrukerType, Periode};

impl From<Periode> for InternUtgangHendelse<Input> {
    fn from(periode: Periode) -> Self {
        match periode.avsluttet {
            None => InternUtgangHendelse::new(
                UtgangHendelseType::Startet,
                ArbeidssoekerperiodeId::from(periode.id),
                periode.startet.tidspunkt,
                periode.startet.utfoert_av.bruker_type.into(),
                None,
            ),
            Some(avsluttet) => InternUtgangHendelse::new(
                UtgangHendelseType::Stoppet,
                ArbeidssoekerperiodeId::from(periode.id),
                avsluttet.tidspunkt,
                avsluttet.utfoert_av.bruker_type.into(),
                None,
            ),
        }
    }
}

impl From<&Periode> for PeriodeRad {
    fn from(value: &Periode) -> Self {
        let sist_oppdatert = value
            .avsluttet
            .as_ref()
            .map(|a| a.tidspunkt)
            .unwrap_or(value.startet.tidspunkt);

        PeriodeRad {
            id: value.id.into(),
            arbeidssoeker_id: None,
            trenger_kontroll: false,
            stoppet: value.avsluttet.is_some(),
            sist_oppdatert,
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

impl From<PeriodeTopicBrukerType> for BrukerType {
    fn from(value: PeriodeTopicBrukerType) -> Self {
        match value {
            PeriodeTopicBrukerType::Sluttbruker => BrukerType::Sluttbruker,
            PeriodeTopicBrukerType::Veileder => BrukerType::Veileder,
            PeriodeTopicBrukerType::System => BrukerType::System,
            PeriodeTopicBrukerType::Udefinert => BrukerType::Udefinert,
            PeriodeTopicBrukerType::UkjentVerdi => BrukerType::UkjentVerdi,
        }
    }
}
