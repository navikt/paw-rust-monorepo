//! HTML for toppfeltet og hver fane. Maud escaper tekst fra databasen
//! (topic-navn, nøkkel-ID, meldingstype).

use chrono::{DateTime, SecondsFormat, Utc};
use maud::{Markup, html};

use crate::features::innlesing::metadata::signature::SignatureStatus;
use crate::features::regler::{alvorlighet::Alvorlighet, regel_katalog::RegelId};

use super::html::{
    Fane, Valg, alvorlighet_merke, alvorlighet_navn, regel_navn, tidspunkt_tekst, varighet_tekst,
};
use super::sporringer::{
    FunnAntall, FunnRad, MAKS_FUNN_I_LISTE, MAKS_SIGNATURFEIL_I_LISTE, ManglerTraceId, RegelStatus,
    SignaturAntall, Signaturfeil, TopicStatus, Toppfelt,
};

const STREK: &str = "–";

pub fn toppfelt(t: &Toppfelt, sist_oppdatert: DateTime<Utc>) -> Markup {
    let datetime = sist_oppdatert.to_rfc3339_opts(SecondsFormat::Secs, true);
    html! {
        p.sist-oppdatert { "Sist oppdatert " time datetime=(datetime) { (sist_oppdatert.format("%H:%M:%S")) } }
        p.utdatert-varsel role="status" { "Tallene kan være utdaterte." }
        ul.nokkeltall {
            li.aktiv { span.dempet { "Stream-tid" } span.verdi { (tidspunkt_tekst(t.stream_ms)) } }
            li.kritisk class=[(t.kritiske_funn == 0).then_some("null")] {
                span.dempet { "Kritiske funn" } span.verdi { (t.kritiske_funn) }
            }
            li.advarsel class=[(t.advarsler == 0).then_some("null")] {
                span.dempet { "Advarsler" } span.verdi { (t.advarsler) }
            }
            li.aktiv { span.dempet { "Signaturfeil" } span.verdi { (t.signaturfeil) } }
        }
    }
}

pub fn oversikt(regler: &[RegelStatus], topics: &[TopicStatus], stream_ms: Option<i64>) -> Markup {
    let min_stream_ms = topics
        .iter()
        .filter_map(|topic| topic.eldste_stream_ms)
        .min();
    html! {
        h2 { "Regler" }
        @if regler.is_empty() {
            p { "Ingen regler har registrert fremdrift ennå." }
        } @else {
            p.dempet { "Meldinger bak regnes bare for partisjoner regelen har startet på. Eldste stream-tid og fremdrift regnes bare for partisjoner med meldinger igjen å lese." }
            div.tabell { table {
                thead { tr {
                    th { "Regel" } th.tall { "Versjon" } th.tall { "Partisjoner" }
                    th.tall { "Meldinger bak" } th { "Eldste stream-tid" } th.tall { "Fremdrift" }
                    th.tall { "Tid bak" } th.tall { "Funn" }
                } }
                tbody { @for r in regler {
                    // Ajour: regelen er så langt som innlesingen har kommet.
                    @let eldste = if r.ajour { stream_ms } else { r.eldste_stream_ms };
                    @let tid_bak = stream_ms
                        .zip(eldste)
                        .map_or(STREK.to_string(), |(naa, regel)| varighet_tekst(naa - regel));
                    @let fremdrift = if r.ajour {
                        Some(100)
                    } else {
                        fremdrift_prosent(min_stream_ms, stream_ms, eldste)
                    };
                    @let funn_url = Valg { fane: Fane::Funn, regel: Some(r.regel as i16), alvorlighet: None }.url();
                    tr {
                        td { (regel_navn(r.regel)) }
                        td.tall { (r.regelversjon) }
                        td.tall { (r.partisjoner) }
                        td.tall { (r.meldinger_bak) }
                        td { (tidspunkt_tekst(eldste)) }
                        td class="tall fremdrift-celle" {
                            @if let Some(prosent) = fremdrift {
                                @let tekst = format!("{prosent} % ajour");
                                meter class=(if prosent < 90 { "fremdrift etterslep" } else { "fremdrift ajour" })
                                    min="0" max="100" value=(prosent) low="0" high="90" optimum="100"
                                    aria-label=(tekst) {
                                    (tekst)
                                }
                                // Kolonneoverskriften gir konteksten. Skjermlesere får hele
                                // teksten fra meteren, så tallet ved siden av skjules for dem.
                                span.prosent aria-hidden="true" { (prosent) " %" }
                            } @else {
                                (STREK)
                            }
                        }
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

fn fremdrift_prosent(min: Option<i64>, max: Option<i64>, verdi: Option<i64>) -> Option<u8> {
    let (min, max, verdi) = (min?, max?, verdi?);
    if max <= min {
        return None;
    }
    let verdi = i128::from(verdi.clamp(min, max));
    let min = i128::from(min);
    let max = i128::from(max);
    Some(((verdi - min) * 100 / (max - min)) as u8)
}

fn filterlenke(valg: Valg, aktiv: bool, tekst: &str) -> Markup {
    html! { a href=(valg.url()) aria-current=[aktiv.then_some("true")] { (tekst) } }
}

const FUNN_DIAGRAM_BREDDE: u32 = 1_000;

#[derive(Debug, PartialEq, Eq)]
struct FunnDiagramRad {
    regel: RegelId,
    kritisk: i64,
    advarsel: i64,
    info: i64,
    total: i64,
    segmentbredder: [u32; 3],
}

/// Slår sammen regelversjoner og skalerer søylene etter største regel-total.
fn funn_diagram_beregning(antall: &[FunnAntall]) -> Vec<FunnDiagramRad> {
    let mut summer: Vec<FunnDiagramRad> = Vec::new();
    for funn in antall {
        let rad = if let Some(rad) = summer.iter_mut().find(|rad| rad.regel == funn.regel) {
            rad
        } else {
            summer.push(FunnDiagramRad {
                regel: funn.regel,
                kritisk: 0,
                advarsel: 0,
                info: 0,
                total: 0,
                segmentbredder: [0; 3],
            });
            summer.last_mut().expect("la nettopp til en rad")
        };
        let verdi = funn.antall.max(0);
        match funn.alvorlighet {
            Alvorlighet::Kritisk => rad.kritisk = rad.kritisk.saturating_add(verdi),
            Alvorlighet::Advarsel => rad.advarsel = rad.advarsel.saturating_add(verdi),
            Alvorlighet::Info => rad.info = rad.info.saturating_add(verdi),
        }
        rad.total = rad.total.saturating_add(verdi);
    }

    let storste_total = summer.iter().map(|rad| rad.total).max().unwrap_or(0);
    if storste_total > 0 {
        for rad in &mut summer {
            let mut kumulativ = 0_u128;
            let mut forrige_bredde = 0_u32;
            for (indeks, verdi) in [rad.kritisk, rad.advarsel, rad.info]
                .into_iter()
                .enumerate()
            {
                kumulativ += verdi as u128;
                let bredde =
                    (kumulativ * u128::from(FUNN_DIAGRAM_BREDDE) / storste_total as u128) as u32;
                rad.segmentbredder[indeks] = bredde - forrige_bredde;
                forrige_bredde = bredde;
            }
        }
    }
    summer
}

fn funn_diagram(antall: &[FunnAntall]) -> Markup {
    let rader = funn_diagram_beregning(antall);
    html! {
        section.funn-diagram aria-label="Funn per regel" {
            ul.funn-diagram-legend aria-label="Alvorlighetsnivå" {
                li { span.prikk.seg-kritisk aria-hidden="true" {} "Kritisk" }
                li { span.prikk.seg-advarsel aria-hidden="true" {} "Advarsel" }
                li { span.prikk.seg-info aria-hidden="true" {} "Info" }
            }
            div.funn-diagram-rader {
                @for rad in rader {
                    @let etikett = format!(
                        "{}: {} kritisk, {} advarsel, {} info",
                        regel_navn(rad.regel), rad.kritisk, rad.advarsel, rad.info
                    );
                    div.funn-diagram-rad {
                        span.regel { (regel_navn(rad.regel)) }
                        span.tall { (rad.total) }
                        svg width="100%" height="24" viewBox="0 0 1000 24"
                            preserveAspectRatio="none" role="img" aria-label=(etikett) {
                            @let [k, a, i] = rad.segmentbredder;
                            @for (klasse, x, bredde) in [
                                ("seg-kritisk", 0, k),
                                ("seg-advarsel", k, a),
                                ("seg-info", k + a, i),
                            ] {
                                rect class=(klasse) x=(x) y="0" width=(bredde) height="24" {}
                            }
                        }
                    }
                }
            }
        }
    }
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
            (funn_diagram(antall))
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
                a href=(Valg { alvorlighet: Some(a), ..*valg }.url())
                    aria-current=[(valg.alvorlighet == Some(a)).then_some("true")] {
                    (alvorlighet_merke(a))
                }
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
                        td { (alvorlighet_merke(f.alvorlighet)) }
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
            regel: Some(RegelId::StartetErSignert as i16),
            alvorlighet: None,
        };
        let antall = [
            FunnAntall {
                regel: RegelId::StartetErSignert,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Advarsel,
                antall: 4,
            },
            FunnAntall {
                regel: RegelId::StartetErSignert,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Kritisk,
                antall: 2,
            },
        ];
        let html = funn(&valg, &antall, &[]).into_string();
        assert_eq!(html.matches("<td>Startet er signert</td>").count(), 1);
        assert!(
            html.contains(
                r#"<td class="tall">2</td><td class="tall">4</td><td class="tall">0</td>"#
            )
        );
        assert!(html.contains(r#"href="/stats/?fane=funn&amp;regel=3&amp;alvorlighet=kritisk""#));
        assert!(html.contains(r#"aria-current="true""#));
        assert!(html.contains(r#"class="merke merke-kritisk""#));
        assert!(html.contains("Kritisk"));
    }

    #[test]
    fn funn_diagram_skalerer_proporsjonalt_og_slaar_sammen_versjoner() {
        let antall = [
            FunnAntall {
                regel: RegelId::StartetErSignert,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Kritisk,
                antall: 2,
            },
            FunnAntall {
                regel: RegelId::StartetErSignert,
                regelversjon: Regelversjon::new(2),
                alvorlighet: Alvorlighet::Advarsel,
                antall: 4,
            },
            FunnAntall {
                regel: RegelId::StartetGirPeriode,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Info,
                antall: 3,
            },
        ];

        let rader = funn_diagram_beregning(&antall);
        assert_eq!(rader.len(), 2);
        assert_eq!(rader[0].total, 6);
        assert_eq!(rader[0].segmentbredder, [333, 667, 0]);
        assert_eq!(rader[1].total, 3);
        assert_eq!(rader[1].segmentbredder, [0, 0, 500]);
    }

    #[test]
    fn funn_diagram_takler_null_funn_og_en_regel() {
        assert!(funn_diagram_beregning(&[]).is_empty());

        let rader = funn_diagram_beregning(&[FunnAntall {
            regel: RegelId::StartetErSignert,
            regelversjon: Regelversjon::new(1),
            alvorlighet: Alvorlighet::Info,
            antall: 5,
        }]);
        assert_eq!(rader.len(), 1);
        assert_eq!(rader[0].total, 5);
        assert_eq!(rader[0].segmentbredder, [0, 0, FUNN_DIAGRAM_BREDDE]);
    }

    #[test]
    fn generert_html_inneholder_ikke_inline_stil() {
        let toppfelt_html = toppfelt(
            &Toppfelt {
                stream_ms: None,
                kritiske_funn: 0,
                advarsler: 2,
                signaturfeil: 0,
            },
            DateTime::from_timestamp(0, 0).unwrap(),
        )
        .into_string();
        let funn_html = funn(
            &Valg::fane(Fane::Funn),
            &[FunnAntall {
                regel: RegelId::StartetErSignert,
                regelversjon: Regelversjon::new(1),
                alvorlighet: Alvorlighet::Info,
                antall: 1,
            }],
            &[],
        )
        .into_string();

        assert!(!toppfelt_html.contains("style="));
        assert!(toppfelt_html.contains(r#"<time datetime="1970-01-01T00:00:00Z">00:00:00</time>"#));
        assert!(toppfelt_html.contains("Tallene kan være utdaterte."));
        assert!(!funn_html.contains("style="));
    }

    #[test]
    fn oversikt_viser_tid_bak() {
        let html = oversikt(
            &[RegelStatus {
                regel: RegelId::StartetErSignert,
                regelversjon: Regelversjon::new(1),
                partisjoner: 2,
                eldste_stream_ms: Some(1_000),
                meldinger_bak: 7,
                antall_funn: 0,
                ajour: false,
            }],
            &[TopicStatus {
                topic: "topic".into(),
                partisjoner: 1,
                eldste_stream_ms: Some(0),
                nyeste_stream_ms: Some(4_000),
            }],
            Some(4_000),
        )
        .into_string();
        assert!(html.contains(r#"<td class="tall">3s</td>"#));
        assert!(html.contains(r#"<td class="tall">7</td>"#));
        assert!(html.contains(r#"aria-label="25 % ajour""#));
        assert!(html.contains(r#"<span class="prosent" aria-hidden="true">25 %</span>"#));
    }

    #[test]
    fn regel_som_er_ajour_vises_som_fullfort() {
        let html = oversikt(
            &[RegelStatus {
                regel: RegelId::HeleTraceSignert,
                regelversjon: Regelversjon::new(1),
                partisjoner: 8,
                eldste_stream_ms: None,
                meldinger_bak: 0,
                antall_funn: 0,
                ajour: true,
            }],
            &[TopicStatus {
                topic: "stille-topic".into(),
                partisjoner: 1,
                eldste_stream_ms: Some(0),
                nyeste_stream_ms: Some(0),
            }],
            Some(4_000),
        )
        .into_string();
        assert!(html.contains(r#"aria-label="100 % ajour""#), "{html}");
        assert!(html.contains(r#"<td class="tall">0s</td>"#), "{html}");
    }

    #[test]
    fn fremdrift_prosent_har_grenser_og_clamper() {
        assert_eq!(fremdrift_prosent(Some(0), Some(100), Some(97)), Some(97));
        assert_eq!(fremdrift_prosent(None, Some(100), Some(97)), None);
        assert_eq!(fremdrift_prosent(Some(0), Some(100), None), None);
        assert_eq!(fremdrift_prosent(Some(5), Some(5), Some(5)), None);
        assert_eq!(fremdrift_prosent(Some(0), Some(100), Some(-10)), Some(0));
        assert_eq!(fremdrift_prosent(Some(0), Some(100), Some(110)), Some(100));
    }
}
