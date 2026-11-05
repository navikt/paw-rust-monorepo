use std::str::FromStr;

use eksterne_hendelser::bekreftelse::vo::bekreftelsesloesning::Bekreftelsesloesning;
use rdkafka::Message;
use rdkafka::message::{Headers, OwnedMessage};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "kilde")]
pub enum Kilde {
    #[sqlx(rename = "inngang-api")]
    InngangApi,
    #[sqlx(rename = "bekreftelse")]
    Bekreftelse,
    #[sqlx(rename = "bekreftelse-tjeneste")]
    BekreftelseTjeneste,
    #[sqlx(rename = "dolly")]
    Dolly,
    #[sqlx(rename = "ukjent")]
    Ukjent,
    #[sqlx(rename = "ikke_aktuelt")]
    IkkeAktuelt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "system", rename_all = "snake_case")]
pub enum System {
    Arbeidssoekerregisteret,
    Dagpenger,
    FriskmeldtTilArbeidsformidling,
    Ukjent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opphav {
    pub kilde: Kilde,
    pub system: System,
}

impl Opphav {
    pub const IKKE_AKTUELT: Opphav = Opphav {
        kilde: Kilde::IkkeAktuelt,
        system: System::Arbeidssoekerregisteret,
    };
    pub const BEKREFTELSE_TJENESTE: Opphav = Opphav {
        kilde: Kilde::BekreftelseTjeneste,
        system: System::Arbeidssoekerregisteret,
    };

    pub fn bekreftelse(loesning: &Bekreftelsesloesning) -> Opphav {
        Opphav {
            kilde: Kilde::Bekreftelse,
            system: system(loesning),
        }
    }
}

fn system(loesning: &Bekreftelsesloesning) -> System {
    match loesning {
        Bekreftelsesloesning::Arbeidssoekerregisteret => System::Arbeidssoekerregisteret,
        Bekreftelsesloesning::Dagpenger => System::Dagpenger,
        Bekreftelsesloesning::FriskmeldtTilArbeidsformidling => {
            System::FriskmeldtTilArbeidsformidling
        }
        Bekreftelsesloesning::UkjentVerdi => System::Ukjent,
    }
}

/// `Metadata.kilde` on Startet and Avsluttet, as set by the producers in
/// paw-arbeidssoekerregisteret-monorepo-intern: an image reference, an app name,
/// `bekreftelse:<loesning>:<id>`, `bekreftelse_paavegneav_stopp:<loesning>` or `Dolly`.
pub(super) fn opphav_fra_hendelse(kilde: &str) -> Opphav {
    let registeret = |kilde| Opphav {
        kilde,
        system: System::Arbeidssoekerregisteret,
    };
    if kilde.starts_with("bekreftelse:") || kilde.starts_with("bekreftelse_paavegneav_stopp:") {
        let loesning = kilde.split(':').nth(1).unwrap_or_default();
        Opphav::bekreftelse(&Bekreftelsesloesning::from_str(loesning).unwrap_or_default())
    } else if kilde.contains("bekreftelse-tjeneste") {
        Opphav::BEKREFTELSE_TJENESTE
    } else if kilde.contains("api-inngang") {
        registeret(Kilde::InngangApi)
    } else if kilde.eq_ignore_ascii_case("dolly") {
        Opphav {
            kilde: Kilde::Dolly,
            system: System::Arbeidssoekerregisteret,
        }
    } else {
        registeret(Kilde::Ukjent)
    }
}

/// PaaVegneAv has no source in its payload, so the last `source` header is used.
pub(super) fn opphav_fra_header(msg: &OwnedMessage) -> Opphav {
    let loesning = msg
        .headers()
        .and_then(|headers| {
            headers
                .iter()
                .filter(|header| header.key == "source")
                .last()
        })
        .and_then(|header| header.value)
        .map(|value| String::from_utf8_lossy(value).into_owned())
        .and_then(|value| Bekreftelsesloesning::from_str(&value).ok())
        .unwrap_or_default();
    Opphav::bekreftelse(&loesning)
}

#[cfg(test)]
mod tests {
    use rdkafka::message::{Header, OwnedHeaders};

    use super::*;

    fn opphav(kilde: Kilde, system: System) -> Opphav {
        Opphav { kilde, system }
    }

    #[test]
    fn hendelse_kilde_tolkes() {
        use Kilde as K;
        use System as S;
        let tilfeller = [
            (
                "europe-north1-docker.pkg.dev/x/paw/paw-arbeidssokerregisteret-api-inngang:26.1",
                opphav(K::InngangApi, S::Arbeidssoekerregisteret),
            ),
            (
                "paw-arbeidssoekerregisteret-bekreftelse-tjeneste",
                opphav(K::BekreftelseTjeneste, S::Arbeidssoekerregisteret),
            ),
            (
                "bekreftelse:dagpenger:42",
                opphav(K::Bekreftelse, S::Dagpenger),
            ),
            (
                "bekreftelse_paavegneav_stopp:friskmeldt_til_arbeidsformidling",
                opphav(K::Bekreftelse, S::FriskmeldtTilArbeidsformidling),
            ),
            ("bekreftelse:noe_nytt:42", opphav(K::Bekreftelse, S::Ukjent)),
            ("Dolly", opphav(K::Dolly, S::Arbeidssoekerregisteret)),
            ("", opphav(K::Ukjent, S::Arbeidssoekerregisteret)),
        ];
        for (kilde, forventet) in tilfeller {
            assert_eq!(opphav_fra_hendelse(kilde), forventet, "{kilde}");
        }
    }

    #[test]
    fn siste_source_header_gir_system() {
        let headers = OwnedHeaders::new()
            .insert(Header {
                key: "source",
                value: Some(b"ukjent"),
            })
            .insert(Header {
                key: "source",
                value: Some(b"dagpenger"),
            });
        let msg = OwnedMessage::new(
            None,
            None,
            "topic".into(),
            rdkafka::Timestamp::NotAvailable,
            0,
            0,
            Some(headers),
        );
        assert_eq!(
            opphav_fra_header(&msg),
            opphav(Kilde::Bekreftelse, System::Dagpenger)
        );
    }
}
