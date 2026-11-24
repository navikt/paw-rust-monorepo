//! HTML for toppfeltet og hver fane. Tekst fra databasen (topic-navn,
//! nøkkel-ID, meldingstype) escapes. Resten er tall, UUID-er eller faste tekster.

use std::fmt::Write;

use crate::features::innlesing::metadata::signature::SignatureStatus;
use crate::features::regler::{alvorlighet::Alvorlighet, regel_katalog::RegelId};

use super::html::{
    Fane, Valg, alvorlighet_navn, escape_html, regel_navn, tidspunkt_tekst, varighet_tekst,
};
use super::sporringer::{
    FunnAntall, FunnRad, MAKS_FUNN_I_LISTE, MAKS_SIGNATURFEIL_I_LISTE, ManglerTraceId, RegelStatus,
    SignaturAntall, Signaturfeil, TopicStatus, Toppfelt,
};

const STREK: &str = "–";

pub fn toppfelt(t: &Toppfelt) -> String {
    format!(
        r#"<ul class="nokkeltall">
<li><span class="dempet">Stream-tid</span><span class="verdi">{}</span></li>
<li class="kritisk"><span class="dempet">Kritiske funn</span><span class="verdi">{}</span></li>
<li class="advarsel"><span class="dempet">Advarsler</span><span class="verdi">{}</span></li>
<li><span class="dempet">Signaturfeil</span><span class="verdi">{}</span></li>
</ul>"#,
        tidspunkt_tekst(t.stream_ms),
        t.kritiske_funn,
        t.advarsler,
        t.signaturfeil,
    )
}

pub fn oversikt(regler: &[RegelStatus], topics: &[TopicStatus], stream_ms: Option<i64>) -> String {
    let mut html = String::from("<h2>Regler</h2>");
    if regler.is_empty() {
        html.push_str("<p>Ingen regler har registrert fremdrift ennå.</p>");
    } else {
        html.push_str(
            r#"<p class="dempet">Meldinger bak regnes bare for partisjoner regelen har startet på.</p>
<div class="tabell"><table><thead><tr><th>Regel</th><th class="tall">Versjon</th><th class="tall">Partisjoner</th>
<th class="tall">Meldinger bak</th><th>Eldste stream-tid</th><th class="tall">Tid bak</th><th class="tall">Funn</th></tr></thead><tbody>"#,
        );
        for r in regler {
            let tid_bak = stream_ms
                .zip(r.eldste_stream_ms)
                .map_or(STREK.to_string(), |(naa, regel)| {
                    varighet_tekst(naa - regel)
                });
            let _ = write!(
                html,
                r##"<tr><td>{}</td><td class="tall">{}</td><td class="tall">{}</td><td class="tall">{}</td><td>{}</td><td class="tall">{}</td><td class="tall"><a href="{url}" hx-get="{url}" hx-target="#fane" hx-swap="outerHTML" hx-push-url="true">{}</a></td></tr>"##,
                regel_navn(r.regel),
                r.regelversjon,
                r.partisjoner,
                r.meldinger_bak,
                tidspunkt_tekst(r.eldste_stream_ms),
                tid_bak,
                r.antall_funn,
                url = Valg {
                    fane: Fane::Funn,
                    regel: Some(r.regel as i16),
                    alvorlighet: None,
                }
                .url(),
            );
        }
        html.push_str("</tbody></table></div>");
    }
    html.push_str("<h2>Topics</h2>");
    html.push_str(
        r#"<div class="tabell"><table><thead><tr><th>Topic</th><th class="tall">Partisjoner med stream-tid</th>
<th>Eldste stream-tid</th><th>Nyeste stream-tid</th></tr></thead><tbody>"#,
    );
    for t in topics {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="tall">{}</td><td>{}</td><td>{}</td></tr>"#,
            escape_html(&t.topic),
            t.partisjoner,
            tidspunkt_tekst(t.eldste_stream_ms),
            tidspunkt_tekst(t.nyeste_stream_ms),
        );
    }
    html.push_str("</tbody></table></div>");
    html
}

/// Lenke som bytter fanen til et nytt filtervalg.
fn filterlenke(valg: Valg, aktiv: bool, tekst: &str) -> String {
    let url = valg.url();
    let aktiv = if aktiv { r#" aria-current="true""# } else { "" };
    format!(
        r##"<a href="{url}" hx-get="{url}" hx-target="#fane" hx-swap="outerHTML" hx-push-url="true"{aktiv}>{tekst}</a>"##
    )
}

pub fn funn(valg: &Valg, antall: &[FunnAntall], rader: &[FunnRad]) -> String {
    let regel = valg.regel.and_then(RegelId::fra_i16);
    let mut html = String::from("<h2>Antall funn</h2>");
    if antall.is_empty() {
        html.push_str("<p>Ingen funn.</p>");
    } else {
        html.push_str(
            r#"<div class="tabell"><table><thead><tr><th>Regel</th><th class="tall">Versjon</th>"#,
        );
        for a in Alvorlighet::ALLE {
            let _ = write!(html, r#"<th class="tall">{}</th>"#, alvorlighet_navn(a));
        }
        html.push_str("</tr></thead><tbody>");
        let mut forrige = None;
        for rad in antall {
            let nokkel = (rad.regel, rad.regelversjon);
            if forrige == Some(nokkel) {
                continue;
            }
            forrige = Some(nokkel);
            let _ = write!(
                html,
                r#"<tr><td>{}</td><td class="tall">{}</td>"#,
                regel_navn(rad.regel),
                rad.regelversjon
            );
            for a in Alvorlighet::ALLE {
                let n = antall
                    .iter()
                    .filter(|x| (x.regel, x.regelversjon) == nokkel && x.alvorlighet == a)
                    .map(|x| x.antall)
                    .sum::<i64>();
                let _ = write!(html, r#"<td class="tall">{n}</td>"#);
            }
            html.push_str("</tr>");
        }
        html.push_str("</tbody></table></div>");
    }

    html.push_str("<h2>Siste funn</h2><div class=\"filter\" aria-label=\"Regel\">");
    html.push_str(&filterlenke(
        Valg {
            regel: None,
            ..*valg
        },
        regel.is_none(),
        "Alle regler",
    ));
    for r in RegelId::ALLE {
        html.push_str(&filterlenke(
            Valg {
                regel: Some(r as i16),
                ..*valg
            },
            regel == Some(r),
            regel_navn(r),
        ));
    }
    html.push_str("</div><div class=\"filter\" aria-label=\"Alvorlighet\">");
    html.push_str(&filterlenke(
        Valg {
            alvorlighet: None,
            ..*valg
        },
        valg.alvorlighet.is_none(),
        "Alle alvorligheter",
    ));
    for a in Alvorlighet::ALLE {
        html.push_str(&filterlenke(
            Valg {
                alvorlighet: Some(a),
                ..*valg
            },
            valg.alvorlighet == Some(a),
            alvorlighet_navn(a),
        ));
    }
    html.push_str("</div>");

    if rader.is_empty() {
        html.push_str("<p>Ingen funn med dette filteret.</p>");
        return html;
    }
    let _ = write!(
        html,
        r#"<p class="dempet">Maks {MAKS_FUNN_I_LISTE}, mest alvorlige og nyeste først.</p>
<div class="tabell"><table><thead><tr><th>Alvorlighet</th><th>Regel</th><th class="tall">Versjon</th><th class="tall">Kode</th>
<th>Topic</th><th class="tall">Partisjon</th><th class="tall">Offset</th><th>Tidspunkt (Kafka)</th><th>Meldingstype</th><th>Trace-ID</th></tr></thead><tbody>"#
    );
    for f in rader {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td>{}</td><td class="tall">{}</td><td class="tall">{}</td><td>{}</td><td class="tall">{}</td><td class="tall">{}</td><td>{}</td><td>{}</td><td class="mono">{}</td></tr>"#,
            alvorlighet_navn(f.alvorlighet),
            regel_navn(f.regel),
            f.regelversjon,
            f.kode,
            escape_html(&f.topic),
            f.record_partition,
            f.record_offset,
            tidspunkt_tekst(f.record_timestamp_ms),
            f.meldingstype
                .as_deref()
                .map_or(STREK.to_string(), escape_html),
            f.trace_id.map_or(STREK.to_string(), |t| t.to_string()),
        );
    }
    html.push_str("</tbody></table></div>");
    html
}

fn signatur_status_tekst(status: SignatureStatus) -> &'static str {
    match status {
        SignatureStatus::Valid => "Gyldig",
        SignatureStatus::SignatureMismatch => "Feil signatur",
        SignatureStatus::MalformedSignature => "Ugyldig signaturformat",
        SignatureStatus::KeyRevoked => "Nøkkel trukket tilbake",
        SignatureStatus::PayloadTooLarge => "For stor payload",
        SignatureStatus::MissingSignature => "Mangler signatur",
        SignatureStatus::UnsignedBeforeRequired => "Usignert før påkrevd",
        SignatureStatus::UnknownKey => "Ukjent nøkkel",
        SignatureStatus::NoRecordTimestamp => "Mangler tidsstempel",
        SignatureStatus::TechnicalError => "Teknisk feil",
    }
}

fn signatur_problem(status: SignatureStatus) -> &'static str {
    match status {
        SignatureStatus::Valid | SignatureStatus::UnsignedBeforeRequired => "",
        SignatureStatus::SignatureMismatch => "Signaturen stemmer ikke med innholdet og nøkkelen",
        SignatureStatus::MalformedSignature => "Signaturen kunne ikke leses",
        SignatureStatus::KeyRevoked => "Nøkkelen var trukket tilbake da meldingen ble sendt",
        SignatureStatus::PayloadTooLarge => "Meldingen er for stor til å valideres",
        SignatureStatus::MissingSignature => {
            "Signatur eller nøkkel-ID mangler etter at signatur ble påkrevd"
        }
        SignatureStatus::UnknownKey => "Nøkkel-ID er ukjent eller ugyldig",
        SignatureStatus::NoRecordTimestamp => {
            "Mangler tidsstempel, så vi kan ikke avgjøre om signatur var påkrevd \
             eller om nøkkelen var gyldig"
        }
        SignatureStatus::TechnicalError => "Feil i nøkkeloppsettet. Signaturen ble ikke sjekket",
    }
}

pub fn signatur(antall: &[SignaturAntall], feil: &[Signaturfeil]) -> String {
    let mut html = String::from(
        "<h2>Signaturfeil per topic</h2>\
         <p class=\"dempet\">Usignerte meldinger fra før signatur ble påkrevd regnes ikke som feil.</p>",
    );
    if antall.is_empty() {
        html.push_str("<p>Ingen meldinger har feilet signaturvalidering.</p>");
        return html;
    }
    html.push_str(
        r#"<div class="tabell"><table><thead><tr><th>Topic</th><th>Status</th><th class="tall">Antall</th></tr></thead><tbody>"#,
    );
    for a in antall {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td>{}</td><td class="tall">{}</td></tr>"#,
            escape_html(&a.topic),
            signatur_status_tekst(a.signature_status),
            a.antall,
        );
    }
    let _ = write!(
        html,
        r#"</tbody></table></div><h2>Siste signaturfeil</h2><p class="dempet">De {MAKS_SIGNATURFEIL_I_LISTE} siste, nyeste først.</p>
<div class="tabell"><table><thead><tr><th>Topic</th><th class="tall">Partisjon</th><th class="tall">Offset</th><th>Tidspunkt (Kafka)</th>
<th>Status</th><th>Problem</th><th>Nøkkel-ID</th><th>Trace-ID</th></tr></thead><tbody>"#
    );
    for f in feil {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="tall">{}</td><td class="tall">{}</td><td>{}</td><td>{}</td><td class="bryt">{}</td><td>{}</td><td class="mono">{}</td></tr>"#,
            escape_html(&f.topic),
            f.record_partition,
            f.record_offset,
            tidspunkt_tekst(f.record_timestamp_ms),
            signatur_status_tekst(f.signature_status),
            signatur_problem(f.signature_status),
            f.key_id.as_deref().map_or(STREK.to_string(), escape_html),
            f.trace_id.map_or(STREK.to_string(), |t| t.to_string()),
        );
    }
    html.push_str("</tbody></table></div>");
    html
}

pub fn trace(rader: &[ManglerTraceId]) -> String {
    let mut html = String::from("<h2>Meldinger uten trace_id</h2>");
    if rader.is_empty() {
        html.push_str("<p>Alle meldinger har trace_id.</p>");
        return html;
    }
    html.push_str(
        r#"<div class="tabell"><table><thead><tr><th>Topic</th><th class="tall">Antall</th><th>Siste tidspunkt (Kafka)</th></tr></thead><tbody>"#,
    );
    for r in rader {
        let _ = write!(
            html,
            r#"<tr><td>{}</td><td class="tall">{}</td><td>{}</td></tr>"#,
            escape_html(&r.topic),
            r.antall,
            tidspunkt_tekst(r.siste_timestamp_ms),
        );
    }
    html.push_str("</tbody></table></div>");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::regler::regelversjon::Regelversjon;

    #[test]
    fn tekst_fra_databasen_escapes() {
        let html = signatur(
            &[SignaturAntall {
                topic: "<b>".into(),
                signature_status: SignatureStatus::UnknownKey,
                antall: 1,
            }],
            &[Signaturfeil {
                topic: "<b>".into(),
                record_partition: 0,
                record_offset: 1,
                record_timestamp_ms: None,
                signature_status: SignatureStatus::SignatureMismatch,
                key_id: Some("<i>".into()),
                trace_id: None,
            }],
        );
        assert!(!html.contains("<b>"));
        assert!(!html.contains("<i>"));
        assert!(html.contains("&lt;i&gt;"));

        let html = trace(&[ManglerTraceId {
            topic: "<script>".into(),
            antall: 3,
            siste_timestamp_ms: Some(0),
        }]);
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn funn_viser_filtre_og_samler_antall_per_regel() {
        let valg = Valg {
            fane: Fane::Funn,
            regel: Some(RegelId::MeldingHarTraceId as i16),
            alvorlighet: None,
        };
        let antall = [
            FunnAntall {
                regel: RegelId::MeldingHarTraceId,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Advarsel,
                antall: 4,
            },
            FunnAntall {
                regel: RegelId::MeldingHarTraceId,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Kritisk,
                antall: 2,
            },
        ];
        let html = funn(&valg, &antall, &[]);
        assert_eq!(html.matches("<td>Melding har trace_id</td>").count(), 1);
        assert!(
            html.contains(
                r#"<td class="tall">2</td><td class="tall">4</td><td class="tall">0</td>"#
            )
        );
        assert!(html.contains(r#"href="/stats/?fane=funn&amp;regel=3&amp;alvorlighet=kritisk""#));
        assert!(html.contains(r#"aria-current="true">Melding har trace_id</a>"#));
    }

    #[test]
    fn oversikt_viser_tid_bak() {
        let html = oversikt(
            &[RegelStatus {
                regel: RegelId::MeldingHarTraceId,
                regelversjon: Regelversjon::new(1),
                partisjoner: 2,
                eldste_stream_ms: Some(1_000),
                meldinger_bak: 7,
                antall_funn: 0,
            }],
            &[],
            Some(4_000),
        );
        assert!(html.contains(r#"<td class="tall">3s</td>"#));
        assert!(html.contains(r#"<td class="tall">7</td>"#));
    }
}
