//! Felles byggeklosser for siden. Maud escaper alt som settes inn med `( … )`.

use std::fmt::Write;

use maud::{DOCTYPE, Markup, html};
use serde::Deserialize;

use crate::features::regler::{alvorlighet::Alvorlighet, regel_katalog::RegelId};

use super::sporringer::Toppfelt;

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
    regel.navn()
}

/// Regelnavn som kan åpnes for å vise hva regelen sjekker.
pub fn regel_med_hjelp(regel: RegelId) -> Markup {
    html! {
        details.regel-hjelp {
            summary { (regel.navn()) }
            p { (regel.beskrivelse()) }
        }
    }
}

/// Utfallskoden med kort tekst, for eksempel «3 Ulikt tidspunkt».
pub fn kode_tekst(regel: RegelId, kode: i16) -> String {
    match regel.kode_tekst(kode) {
        Some(tekst) => format!("{kode} {tekst}"),
        None => kode.to_string(),
    }
}

pub fn alvorlighet_navn(alvorlighet: Alvorlighet) -> &'static str {
    match alvorlighet {
        Alvorlighet::Info => "Info",
        Alvorlighet::Advarsel => "Advarsel",
        Alvorlighet::Kritisk => "Kritisk",
    }
}

/// Vis alvorlighetsnivå med både tekst og et formbasert ikon.
pub fn alvorlighet_merke(alvorlighet: Alvorlighet) -> Markup {
    let ikon = match alvorlighet {
        Alvorlighet::Info => "●",
        Alvorlighet::Advarsel => "▲",
        Alvorlighet::Kritisk => "■",
    };
    html! {
        span class=(format!("merke merke-{}", alvorlighet.as_str())) {
            span aria-hidden="true" { (ikon) }
            " "
            (alvorlighet_navn(alvorlighet))
        }
    }
}

/// Fargetema. `Auto` følger nettleseren.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tema {
    Lys,
    Mork,
    #[default]
    Auto,
}

impl Tema {
    pub const ALLE: [Tema; 3] = [Self::Lys, Self::Mork, Self::Auto];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lys => "lys",
            Self::Mork => "mork",
            Self::Auto => "auto",
        }
    }

    fn navn(self) -> &'static str {
        match self {
            Self::Lys => "Lys",
            Self::Mork => "Mørk",
            Self::Auto => "Auto",
        }
    }

    /// Leser `tema` fra en `Cookie`-header. Manglende eller ukjent verdi gir `Auto`.
    pub fn fra_cookie_header(header: Option<&str>) -> Self {
        header
            .into_iter()
            .flat_map(|h| h.split(';'))
            .filter_map(|par| par.trim().split_once('='))
            .find(|(navn, _)| *navn == "tema")
            .and_then(|(_, verdi)| Self::ALLE.into_iter().find(|t| t.as_str() == verdi))
            .unwrap_or_default()
    }
}

#[derive(Debug, Deserialize)]
pub struct TemaSkjema {
    pub tema: Tema,
}

fn temavelger(valgt: Tema) -> Markup {
    html! {
        form.tema method="post" action="/stats/tema" {
            fieldset {
                legend { "Tema" }
                @for tema in Tema::ALLE {
                    label {
                        input type="radio" name="tema" value=(tema.as_str()) checked[tema == valgt];
                        " " (tema.navn())
                    }
                }
                button type="submit" { "Bytt tema" }
            }
        }
    }
}

/// Hele siden. Virker uten JavaScript; `oppdater.js` henter bare nye tall.
/// Fanelenkene viser tellerne fra sidelastingen og oppdateres ikke automatisk.
pub fn side(
    valg: &Valg,
    tema: Tema,
    toppfelt_data: &Toppfelt,
    toppfelt: Markup,
    innhold: Markup,
) -> Markup {
    html! {
        (DOCTYPE)
        html lang="nb" data-tema=(tema.as_str()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Internkontroll" }
                link rel="stylesheet" href="/stats/static/app.css";
                script src="/stats/static/oppdater.js" defer {}
            }
            body {
                main {
                    (temavelger(tema))
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
                            @let antall = match fane {
                                Fane::Funn => Some(toppfelt_data.kritiske_funn.saturating_add(toppfelt_data.advarsler)),
                                Fane::Signatur => Some(toppfelt_data.signaturfeil),
                                Fane::Oversikt | Fane::Trace => None,
                            };
                            a href=(Valg::fane(fane).url())
                                aria-current=[(fane == valg.fane).then_some("page")] {
                                (fane.navn())
                                @if let Some(antall) = antall {
                                    span class=(if fane == Fane::Funn && antall > 0 && toppfelt_data.kritiske_funn > 0 { "fanetall kritisk" } else { "fanetall" })
                                        aria-label=(format!("{} {antall}", fane.navn())) {
                                        (antall)
                                    }
                                }
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

/// Side uten faner og automatisk oppdatering, for detaljsider.
pub fn enkel_side(tittel: &str, tema: Tema, innhold: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="nb" data-tema=(tema.as_str()) {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (tittel) " – Internkontroll" }
                link rel="stylesheet" href="/stats/static/app.css";
            }
            body { main { (innhold) } }
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
    fn alvorlighetsmerke_viser_navn_som_tekst() {
        for (alvorlighet, navn) in [
            (Alvorlighet::Info, "Info"),
            (Alvorlighet::Advarsel, "Advarsel"),
            (Alvorlighet::Kritisk, "Kritisk"),
        ] {
            let html = alvorlighet_merke(alvorlighet).into_string();
            assert!(html.contains(navn));
            assert!(html.contains(r#"aria-hidden="true""#));
        }
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
        let toppfelt = Toppfelt {
            stream_ms: None,
            kritiske_funn: 5,
            advarsler: 7,
            signaturfeil: 3,
        };
        let html = side(&valg, Tema::Mork, &toppfelt, html! {}, html! {}).into_string();
        assert!(
            html.contains(r#"<a href="/stats/?fane=signatur" aria-current="page">Signatur<span"#)
        );
        assert_eq!(html.matches("aria-current").count(), 1);
        assert!(html.contains(r#"class="fanetall" aria-label="Signatur 3">3</span>"#));
        assert!(html.contains(r#"class="fanetall kritisk" aria-label="Funn 12">12</span>"#));
        assert!(!html.contains(r#"aria-label="Oversikt 0"#));
        assert!(!html.contains(r#"aria-label="Trace 0"#));
        assert!(html.contains(r#"data-oppdater="/stats/fragment/fane?fane=signatur&amp;regel=3""#));
        assert!(html.contains(r#"<html lang="nb" data-tema="mork">"#));
        assert!(html.contains(r#"value="mork" checked>"#));
        assert_eq!(html.matches("checked").count(), 1);
    }

    #[test]
    fn tema_leses_fra_cookie() {
        assert_eq!(Tema::fra_cookie_header(None), Tema::Auto);
        assert_eq!(Tema::fra_cookie_header(Some("")), Tema::Auto);
        assert_eq!(Tema::fra_cookie_header(Some("tema=ukjent")), Tema::Auto);
        assert_eq!(Tema::fra_cookie_header(Some("tema=<b>")), Tema::Auto);
        assert_eq!(Tema::fra_cookie_header(Some("xtema=lys")), Tema::Auto);
        for tema in Tema::ALLE {
            let header = format!("a=1; tema={}; b=2", tema.as_str());
            assert_eq!(Tema::fra_cookie_header(Some(&header)), tema);
        }
    }
}
