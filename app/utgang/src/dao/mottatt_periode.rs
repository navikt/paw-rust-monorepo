use interne_hendelser::vo::BrukerType;

use crate::dao::perioder::{PeriodeRad, skriv_perioder};
use crate::dao::utgang_hendelse::InternUtgangHendelse;
use crate::dao::utgang_hendelser_logg::skriv_hendelser;
use crate::domain::arbeidssoekerperiode_id::ArbeidssoekerperiodeId;
use crate::domain::utgang_hendelse_type::UtgangHendelseType;
use crate::kafka::periode_deserializer::{BrukerType as KafkaBrukerType, Periode};

pub async fn behandle_mottatte_perioder(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    perioder: &[Periode],
) -> Result<(), sqlx::Error> {
    if perioder.is_empty() {
        return Ok(());
    }

    let hendelser = perioder.iter().map(til_hendelse).collect();
    let periode_rader = perioder.iter().map(til_periode_rad).collect();

    skriv_hendelser(tx, hendelser).await?;
    skriv_perioder(tx, periode_rader).await?;
    Ok(())
}

fn til_hendelse(periode: &Periode) -> InternUtgangHendelse<crate::dao::utgang_hendelse::Input> {
    match &periode.avsluttet {
        None => InternUtgangHendelse::new(
            UtgangHendelseType::Startet,
            ArbeidssoekerperiodeId::from(periode.id),
            periode.startet.tidspunkt,
            konverter_brukertype(&periode.startet.utfoert_av.bruker_type),
            None,
        ),
        Some(avsluttet) => InternUtgangHendelse::new(
            UtgangHendelseType::Stoppet,
            ArbeidssoekerperiodeId::from(periode.id),
            avsluttet.tidspunkt,
            konverter_brukertype(&avsluttet.utfoert_av.bruker_type),
            None,
        ),
    }
}

fn til_periode_rad(periode: &Periode) -> PeriodeRad {
    let sist_oppdatert = periode
        .avsluttet
        .as_ref()
        .map(|a| a.tidspunkt)
        .unwrap_or(periode.startet.tidspunkt);

    PeriodeRad {
        id: ArbeidssoekerperiodeId::from(periode.id),
        arbeidssoeker_id: None,
        trenger_kontroll: false,
        stoppet: periode.avsluttet.is_some(),
        sist_oppdatert,
    }
}

fn konverter_brukertype(bt: &KafkaBrukerType) -> BrukerType {
    match bt {
        KafkaBrukerType::Sluttbruker => BrukerType::Sluttbruker,
        KafkaBrukerType::Veileder => BrukerType::Veileder,
        KafkaBrukerType::System => BrukerType::System,
        KafkaBrukerType::Udefinert => BrukerType::Udefinert,
        KafkaBrukerType::UkjentVerdi => BrukerType::UkjentVerdi,
    }
}
