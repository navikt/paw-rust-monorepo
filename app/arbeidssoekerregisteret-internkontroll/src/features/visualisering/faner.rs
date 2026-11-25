//! HTML for toppfeltet og hver fane. Maud escaper tekst fra databasen
//! (topic-navn, nøkkel-ID, meldingstype).

use maud::{Markup, html};

use crate::features::innlesing::metadata::signature::SignatureStatus;
use crate::features::regler::{alvorlighet::Alvorlighet, regel_katalog::RegelId};

use super::html::{Fane, Valg, alvorlighet_navn, regel_navn, tidspunkt_tekst, varighet_tekst};
use super::sporringer::{
    FunnAntall, FunnRad, MAKS_FUNN_I_LISTE, MAKS_SIGNATURFEIL_I_LISTE, ManglerTraceId, RegelStatus,
    SignaturAntall, Signaturfeil, TopicStatus, Toppfelt,
};

const STREK: &str = "–";

pub fn toppfelt(t: &Toppfelt) -> Markup {
    html! {
        ul.nokkeltall {
            li { span.dempet { "Stream-tid" } span.verdi { (tidspunkt_tekst(t.stream_ms)) } }
            li.kritisk { span.dempet { "Kritiske funn" } span.verdi { (t.kritiske_funn) } }
            li.advarsel { span.dempet { "Advarsler" } span.verdi { (t.advarsler) } }
            li { span.dempet { "Signaturfeil" } span.verdi { (t.signaturfeil) } }
        }
    }
}

pub fn oversikt(regler: &[RegelStatus], topics: &[TopicStatus], stream_ms: Option<i64>) -> Markup {
    html! {
        h2 { "Regler" }
        @if regler.is_empty() {
            p { "Ingen regler har registrert fremdrift ennå." }
        } @else {
            p.dempet { "Meldinger bak regnes bare for partisjoner regelen har startet på." }
            div.tabell { table {
                thead { tr {
                    th { "Regel" } th.tall { "Versjon" } th.tall { "Partisjoner" }
                    th.tall { "Meldinger bak" } th { "Eldste stream-tid" } th.tall { "Tid bak" } th.tall { "Funn" }
                } }
                tbody { @for r in regler {
                    @let tid_bak = stream_ms
                        .zip(r.eldste_stream_ms)
                        .map_or(STREK.to_string(), |(naa, regel)| varighet_tekst(naa - regel));
                    @let funn_url = Valg { fane: Fane::Funn, regel: Some(r.regel as i16), alvorlighet: None }.url();
                    tr {
                        td { (regel_navn(r.regel)) }
                        td.tall { (r.regelversjon) }
                        td.tall { (r.partisjoner) }
                        td.tall { (r.meldinger_bak) }
                        td { (tidspunkt_tekst(r.eldste_stream_ms)) }
                        td.tall { (tid_bak) }
                        td.tall { a href=(funn_url) { (r.antall_funn) } }
                    }
                } }
            } }
        }
        h2 { "Topics" }
        div.tabell { table {
            thead { tr {
                th { "Topic" } th.tall { "Partisjoner med stream-tid" }
                th { "Eldste stream-tid" } th { "Nyeste stream-tid" }
            } }
            tbody { @for t in topics {
                tr {
                    td { (t.topic) }
                    td.tall { (t.partisjoner) }
                    td { (tidspunkt_tekst(t.eldste_stream_ms)) }
                    td { (tidspunkt_tekst(t.nyeste_stream_ms)) }
                }
            } }
        } }
    }
}

fn filterlenke(valg: Valg, aktiv: bool, tekst: &str) -> Markup {
    html! { a href=(valg.url()) aria-current=[aktiv.then_some("true")] { (tekst) } }
}

pub fn funn(valg: &Valg, antall: &[FunnAntall], rader: &[FunnRad]) -> Markup {
    let regel = valg.regel.and_then(RegelId::fra_i16);
    let mut regelversjoner: Vec<_> = antall.iter().map(|a| (a.regel, a.regelversjon)).collect();
    regelversjoner.dedup();
    let antall_for = |nokkel, a| {
        antall
            .iter()
            .filter(|x| (x.regel, x.regelversjon) == nokkel && x.alvorlighet == a)
            .map(|x| x.antall)
            .sum::<i64>()
    };
    html! {
        h2 { "Antall funn" }
        @if antall.is_empty() {
            p { "Ingen funn." }
        } @else {
            div.tabell { table {
                thead { tr {
                    th { "Regel" } th.tall { "Versjon" }
                    @for a in Alvorlighet::ALLE { th.tall { (alvorlighet_navn(a)) } }
                } }
                tbody { @for nokkel in &regelversjoner {
                    tr {
                        td { (regel_navn(nokkel.0)) }
                        td.tall { (nokkel.1) }
                        @for a in Alvorlighet::ALLE { td.tall { (antall_for(*nokkel, a)) } }
                    }
                } }
            } }
        }

        h2 { "Siste funn" }
        nav.filter aria-label="Regel" {
            (filterlenke(Valg { regel: None, ..*valg }, regel.is_none(), "Alle regler"))
            @for r in RegelId::ALLE {
                (filterlenke(Valg { regel: Some(r as i16), ..*valg }, regel == Some(r), regel_navn(r)))
            }
        }
        nav.filter aria-label="Alvorlighet" {
            (filterlenke(Valg { alvorlighet: None, ..*valg }, valg.alvorlighet.is_none(), "Alle alvorligheter"))
            @for a in Alvorlighet::ALLE {
                (filterlenke(Valg { alvorlighet: Some(a), ..*valg }, valg.alvorlighet == Some(a), alvorlighet_navn(a)))
            }
        }
        @if rader.is_empty() {
            p { "Ingen funn med dette filteret." }
        } @else {
            p.dempet { "Maks " (MAKS_FUNN_I_LISTE) ", mest alvorlige og nyeste først." }
            div.tabell { table {
                thead { tr {
                    th { "Alvorlighet" } th { "Regel" } th.tall { "Versjon" } th.tall { "Kode" }
                    th { "Topic" } th.tall { "Partisjon" } th.tall { "Offset" }
                    th { "Tidspunkt (Kafka)" } th { "Meldingstype" } th { "Trace-ID" }
                } }
                tbody { @for f in rader {
                    tr {
                        td { (alvorlighet_navn(f.alvorlighet)) }
                        td { (regel_navn(f.regel)) }
                        td.tall { (f.regelversjon) }
                        td.tall { (f.kode) }
                        td { (f.topic) }
                        td.tall { (f.record_partition) }
                        td.tall { (f.record_offset) }
                        td { (tidspunkt_tekst(f.record_timestamp_ms)) }
                        td { (f.meldingstype.as_deref().unwrap_or(STREK)) }
                        td.mono { (f.trace_id.map_or(STREK.to_string(), |t| t.to_string())) }
                    }
                } }
            } }
        }
    }
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

pub fn signatur(antall: &[SignaturAntall], feil: &[Signaturfeil]) -> Markup {
    html! {
        h2 { "Signaturfeil per topic" }
        p.dempet { "Usignerte meldinger fra før signatur ble påkrevd regnes ikke som feil." }
        @if antall.is_empty() {
            p { "Ingen meldinger har feilet signaturvalidering." }
        } @else {
            div.tabell { table {
                thead { tr { th { "Topic" } th { "Status" } th.tall { "Antall" } } }
                tbody { @for a in antall {
                    tr {
                        td { (a.topic) }
                        td { (signatur_status_tekst(a.signature_status)) }
                        td.tall { (a.antall) }
                    }
                } }
            } }
            h2 { "Siste signaturfeil" }
            p.dempet { "De " (MAKS_SIGNATURFEIL_I_LISTE) " siste, nyeste først." }
            div.tabell { table {
                thead { tr {
                    th { "Topic" } th.tall { "Partisjon" } th.tall { "Offset" } th { "Tidspunkt (Kafka)" }
                    th { "Status" } th { "Problem" } th { "Nøkkel-ID" } th { "Trace-ID" }
                } }
                tbody { @for f in feil {
                    tr {
                        td { (f.topic) }
                        td.tall { (f.record_partition) }
                        td.tall { (f.record_offset) }
                        td { (tidspunkt_tekst(f.record_timestamp_ms)) }
                        td { (signatur_status_tekst(f.signature_status)) }
                        td.bryt { (signatur_problem(f.signature_status)) }
                        td { (f.key_id.as_deref().unwrap_or(STREK)) }
                        td.mono { (f.trace_id.map_or(STREK.to_string(), |t| t.to_string())) }
                    }
                } }
            } }
        }
    }
}

pub fn trace(rader: &[ManglerTraceId]) -> Markup {
    html! {
        h2 { "Meldinger uten trace_id" }
        @if rader.is_empty() {
            p { "Alle meldinger har trace_id." }
        } @else {
            div.tabell { table {
                thead { tr { th { "Topic" } th.tall { "Antall" } th { "Siste tidspunkt (Kafka)" } } }
                tbody { @for r in rader {
                    tr {
                        td { (r.topic) }
                        td.tall { (r.antall) }
                        td { (tidspunkt_tekst(r.siste_timestamp_ms)) }
                    }
                } }
            } }
        }
    }
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
        )
        .into_string();
        assert!(!html.contains("<b>"));
        assert!(!html.contains("<i>"));
        assert!(html.contains("&lt;i&gt;"));

        let html = trace(&[ManglerTraceId {
            topic: "<script>".into(),
            antall: 3,
            siste_timestamp_ms: Some(0),
        }])
        .into_string();
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
        let html = funn(&valg, &antall, &[]).into_string();
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
        )
        .into_string();
        assert!(html.contains(r#"<td class="tall">3s</td>"#));
        assert!(html.contains(r#"<td class="tall">7</td>"#));
    }
}
