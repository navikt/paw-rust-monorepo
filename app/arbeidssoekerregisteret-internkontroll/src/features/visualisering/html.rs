//! Felles byggeklosser for siden. Maud escaper alt som settes inn med `( … )`.

use std::fmt::Write;

use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;

use crate::features::regler::{alvorlighet::Alvorlighet, regel_katalog::RegelId};

/// Sekunder mellom hver automatiske oppdatering.
const FANE_INTERVALL: u32 = 30;
const TOPPFELT_INTERVALL: u32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fane {
    #[default]
    Oversikt,
    Funn,
    Signatur,
    Trace,
}

impl Fane {
    pub const ALLE: [Fane; 4] = [Self::Oversikt, Self::Funn, Self::Signatur, Self::Trace];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Oversikt => "oversikt",
            Self::Funn => "funn",
            Self::Signatur => "signatur",
            Self::Trace => "trace",
        }
    }

    pub fn navn(self) -> &'static str {
        match self {
            Self::Oversikt => "Oversikt",
            Self::Funn => "Funn",
            Self::Signatur => "Signatur",
            Self::Trace => "Trace",
        }
    }
}

/// Hva brukeren har valgt: fane og filtre. Leses fra query-strengen, og
/// ukjente verdier avvises med 400 før noe hentes fra databasen.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Valg {
    #[serde(default)]
    pub fane: Fane,
    pub regel: Option<i16>,
    pub alvorlighet: Option<Alvorlighet>,
}

impl Valg {
    pub fn fane(fane: Fane) -> Self {
        Self {
            fane,
            ..Self::default()
        }
    }

    /// Bygges bare av enum-verdier og tall, aldri av tekst fra requesten.
    fn query(&self) -> String {
        let mut q = format!("fane={}", self.fane.as_str());
        if let Some(regel) = self.regel {
            let _ = write!(q, "&regel={regel}");
        }
        if let Some(alvorlighet) = self.alvorlighet {
            let _ = write!(q, "&alvorlighet={}", alvorlighet.as_str());
        }
        q
    }

    /// Lenke til hele siden med dette valget.
    pub fn url(&self) -> String {
        format!("/stats/?{}", self.query())
    }

    /// Bare faneinnholdet, for automatisk oppdatering.
    pub fn fragment_url(&self) -> String {
        format!("/stats/fragment/fane?{}", self.query())
    }
}

pub fn tidspunkt_tekst(ms: Option<i64>) -> String {
    ms.and_then(chrono::DateTime::from_timestamp_millis)
        .map_or("–".to_string(), |t| {
            t.format("%Y-%m-%d %H:%M:%S%.3f UTC").to_string()
        })
}

pub fn varighet_tekst(ms: i64) -> String {
    let sekunder = ms.max(0) / 1_000;
    let dager = sekunder / 86_400;
    let timer = sekunder / 3_600 % 24;
    let minutter = sekunder / 60 % 60;
    let sekunder = sekunder % 60;
    if dager > 0 {
        format!("{dager}d {timer:02}t {minutter:02}m")
    } else if timer > 0 {
        format!("{timer}t {minutter:02}m {sekunder:02}s")
    } else if minutter > 0 {
        format!("{minutter}m {sekunder:02}s")
    } else {
        format!("{sekunder}s")
    }
}

pub fn regel_navn(regel: RegelId) -> &'static str {
    match regel {
        RegelId::HeleTraceSignert => "Hele trace er signert",
        RegelId::IngenPeriodeUtenStartet => "Ingen periode uten startet",
        RegelId::MeldingHarTraceId => "Melding har trace_id",
    }
}

pub fn alvorlighet_navn(alvorlighet: Alvorlighet) -> &'static str {
    match alvorlighet {
        Alvorlighet::Info => "Info",
        Alvorlighet::Advarsel => "Advarsel",
        Alvorlighet::Kritisk => "Kritisk",
    }
}

/// Hele siden. Virker uten JavaScript; `oppdater.js` henter bare nye tall.
pub fn side(valg: &Valg, toppfelt: Markup, innhold: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="nb" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Internkontroll" }
                link rel="stylesheet" href="/stats/static/app.css";
                script src="/stats/static/oppdater.js" defer {}
            }
            body {
                main {
                    h1 { "Internkontroll" }
                    p {
                        "Internkontroll for arbeidssøkerregisteret. Siden viser hvor langt innlesing "
                        "og regelkjøring har kommet, og hvilke avvik som er funnet. Tallene oppdateres automatisk."
                    }
                    section #toppfelt aria-label="Nøkkeltall"
                        data-oppdater="/stats/fragment/toppfelt" data-intervall=(TOPPFELT_INTERVALL) {
                        (toppfelt)
                    }
                    nav.faner aria-label="Faner" {
                        @for fane in Fane::ALLE {
                            a href=(Valg::fane(fane).url())
                                aria-current=[(fane == valg.fane).then_some("page")] {
                                (fane.navn())
                            }
                        }
                    }
                    section #fane data-oppdater=(valg.fragment_url()) data-intervall=(FANE_INTERVALL) {
                        (innhold)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varighet_formateres_etter_storrelse() {
        assert_eq!(varighet_tekst(0), "0s");
        assert_eq!(varighet_tekst(45_000), "45s");
        assert_eq!(varighet_tekst(185_000), "3m 05s");
        assert_eq!(varighet_tekst(7_623_000), "2t 07m 03s");
        assert_eq!(varighet_tekst(349_320_000), "4d 01t 02m");
        assert_eq!(varighet_tekst(-1), "0s");
    }

    #[test]
    fn tidspunkt_formateres_i_utc() {
        assert_eq!(tidspunkt_tekst(Some(0)), "1970-01-01 00:00:00.000 UTC");
        assert_eq!(tidspunkt_tekst(None), "–");
    }

    #[test]
    fn url_kombinerer_filtre() {
        let valg = Valg {
            fane: Fane::Funn,
            regel: Some(3),
            alvorlighet: Some(Alvorlighet::Kritisk),
        };
        assert_eq!(valg.url(), "/stats/?fane=funn&regel=3&alvorlighet=kritisk");
        assert_eq!(
            valg.fragment_url(),
            "/stats/fragment/fane?fane=funn&regel=3&alvorlighet=kritisk"
        );
    }

    #[test]
    fn side_markerer_aktiv_fane_og_escaper_url() {
        let valg = Valg {
            fane: Fane::Signatur,
            regel: Some(3),
            alvorlighet: None,
        };
        let html = side(&valg, html! {}, html! {}).into_string();
        assert!(
            html.contains(r#"<a href="/stats/?fane=signatur" aria-current="page">Signatur</a>"#)
        );
        assert_eq!(html.matches("aria-current").count(), 1);
        assert!(html.contains(r#"data-oppdater="/stats/fragment/fane?fane=signatur&amp;regel=3""#));
    }
}
