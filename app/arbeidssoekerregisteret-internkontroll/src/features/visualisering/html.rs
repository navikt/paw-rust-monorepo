//! HTML-byggeklosser. All tekst som ikke er en fast streng i koden, skal
//! gjennom [`escape_html`] før den havner i HTML-en.

use std::fmt::Write;

use serde::Deserialize;

use crate::features::regler::{alvorlighet::Alvorlighet, regel_katalog::RegelId};

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

    /// URL til valget, klar til bruk i et HTML-attributt. Bygges bare av
    /// enum-verdier og tall, aldri av tekst fra requesten.
    pub fn url(&self) -> String {
        let mut url = format!("/stats/?fane={}", self.fane.as_str());
        if let Some(regel) = self.regel {
            let _ = write!(url, "&amp;regel={regel}");
        }
        if let Some(alvorlighet) = self.alvorlighet {
            let _ = write!(url, "&amp;alvorlighet={}", alvorlighet.as_str());
        }
        url
    }
}

pub fn escape_html(s: &str) -> String {
    let mut ut = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => ut.push_str("&amp;"),
            '<' => ut.push_str("&lt;"),
            '>' => ut.push_str("&gt;"),
            '"' => ut.push_str("&quot;"),
            '\'' => ut.push_str("&#39;"),
            _ => ut.push(c),
        }
    }
    ut
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

/// Hele siden. Toppfeltet hentes av HTMX, fanen rendres med en gang.
pub fn side(fane_seksjon: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="nb">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="htmx-config" content='{{"includeIndicatorStyles":false,"historyCacheSize":0}}'>
    <title>Internkontroll</title>
    <link rel="stylesheet" href="/stats/static/app.css">
    <script src="/stats/static/htmx.min.js"></script>
</head>
<body>
    <main>
        <h1>Internkontroll</h1>
        <p>Internkontroll for arbeidssøkerregisteret. Siden viser hvor langt innlesing og regelkjøring har kommet, og hvilke avvik som er funnet. Tallene oppdateres automatisk.</p>
        <section id="toppfelt" aria-label="Nøkkeltall" hx-get="/stats/toppfelt" hx-trigger="load, every 60s">
            <p class="dempet">Laster nøkkeltall …</p>
        </section>
        {fane_seksjon}
    </main>
</body>
</html>"#
    )
}

/// Fanevelger og innholdet i valgt fane. Byttes ut i sin helhet ved fanebytte,
/// filtervalg og polling, slik at URL-en for pollingen alltid følger valget.
pub fn fane_seksjon(valg: &Valg, innhold: &str) -> String {
    let mut html = format!(
        r#"<section id="fane" hx-get="{}" hx-trigger="every 30s" hx-swap="outerHTML">
<nav class="faner" aria-label="Faner">"#,
        valg.url()
    );
    for fane in Fane::ALLE {
        let url = Valg::fane(fane).url();
        let aktiv = if fane == valg.fane {
            r#" aria-current="page""#
        } else {
            ""
        };
        let _ = write!(
            html,
            r##"<a href="{url}" hx-get="{url}" hx-target="#fane" hx-swap="outerHTML" hx-push-url="true"{aktiv}>{}</a>"##,
            fane.navn()
        );
    }
    let _ = write!(html, "</nav>\n<div>{innhold}</div>\n</section>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_escaper_alle_spesialtegn() {
        assert_eq!(
            escape_html("<script>&'\"x"),
            "&lt;script&gt;&amp;&#39;&quot;x"
        );
        assert_eq!(escape_html("æøå"), "æøå");
    }

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
    fn url_kombinerer_filtre_og_escaper_skilletegn() {
        let valg = Valg {
            fane: Fane::Funn,
            regel: Some(3),
            alvorlighet: Some(Alvorlighet::Kritisk),
        };
        assert_eq!(
            valg.url(),
            "/stats/?fane=funn&amp;regel=3&amp;alvorlighet=kritisk"
        );
    }

    #[test]
    fn fane_seksjon_markerer_aktiv_fane() {
        let html = fane_seksjon(&Valg::fane(Fane::Signatur), "");
        assert!(html.contains(r#"hx-push-url="true" aria-current="page">Signatur</a>"#));
        assert_eq!(html.matches("aria-current").count(), 1);
    }
}
