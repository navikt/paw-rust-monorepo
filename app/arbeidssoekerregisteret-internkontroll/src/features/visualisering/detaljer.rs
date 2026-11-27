//! Detaljsiden for ett funn: hvilke meldinger regelen brukte, og hva som skilte
//! dem. Fødselsnummer vises aldri, se `pseudonym.rs`.

use maud::{Markup, html};
use serde::Deserialize;

use crate::features::innlesing::metadata::record_metadata_id::RecordMetadataId;
use crate::features::innlesing::metadata::utfoert::UtfoertAvType;
use crate::features::regler::{regel_katalog::RegelId, regelversjon::Regelversjon};

use super::html::{alvorlighet_merke, kode_tekst, tidspunkt_tekst, trace_id_tekst};
use super::pseudonym::Pseudonymer;
use super::sporringer::{DetaljMelding, FunnDetaljer};

const STREK: &str = "–";

/// Nøkkelen til et funn, fra query-strengen.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct FunnNokkel {
    pub regel: i16,
    pub versjon: i32,
    pub melding: RecordMetadataId,
}

impl FunnNokkel {
    pub fn url(regel: RegelId, versjon: Regelversjon, melding: RecordMetadataId) -> String {
        format!(
            "/stats/funn?regel={}&versjon={versjon}&melding={melding}",
            regel as i16
        )
    }
}

/// Én rad i meldingstabellen etter pseudonymisering.
struct VistMelding {
    id: RecordMetadataId,
    kilde: Option<DetaljMelding>,
    person: Option<String>,
    utfoert_av_type: Option<UtfoertAvType>,
    utfoert_av: Option<String>,
}

/// Meldingene i den rekkefølgen funnet lagret dem, med fødselsnummer byttet ut.
/// Personen i første melding blir A, så bokstavene leses likt fra gang til gang.
fn vis_meldinger(
    rekkefolge: &[RecordMetadataId],
    mut hentet: Vec<DetaljMelding>,
) -> Vec<VistMelding> {
    let mut pseudonymer = Pseudonymer::default();
    let mut rader: Vec<VistMelding> = rekkefolge
        .iter()
        .map(|id| {
            let kilde = hentet
                .iter()
                .position(|m| m.id == *id)
                .map(|i| hentet.swap_remove(i));
            let person = kilde
                .as_ref()
                .and_then(|m| m.identitetsnummer.as_deref())
                .map(|fnr| pseudonymer.bokstav(fnr));
            VistMelding {
                id: *id,
                kilde,
                person,
                utfoert_av_type: None,
                utfoert_av: None,
            }
        })
        .collect();
    for rad in &mut rader {
        let Some(m) = &rad.kilde else { continue };
        rad.utfoert_av_type = m.utfoert_av_type;
        rad.utfoert_av = m
            .utfoert_av_type
            .zip(m.utfoert_av_id.as_deref())
            .map(|(av_type, av_id)| pseudonymer.utfoert_av(av_type, av_id));
    }
    rader
}

fn ulike<T: PartialEq>(verdier: impl Iterator<Item = T>) -> bool {
    let verdier: Vec<T> = verdier.collect();
    verdier.windows(2).any(|w| w[0] != w[1])
}

fn kolonne(navn: &str, avviker: bool) -> Markup {
    html! {
        @if avviker {
            th.avvik { (navn) " " span.avvik-merke { "avviker" } }
        } @else {
            th { (navn) }
        }
    }
}

fn utfoert_av_type_tekst(t: UtfoertAvType) -> &'static str {
    match t {
        UtfoertAvType::Udefinert => "Udefinert",
        UtfoertAvType::UkjentVerdi => "Ukjent verdi",
        UtfoertAvType::System => "System",
        UtfoertAvType::Sluttbruker => "Sluttbruker",
        UtfoertAvType::Veileder => "Veileder",
    }
}

pub fn funn_detaljer(
    regel: RegelId,
    versjon: Regelversjon,
    funn: &FunnDetaljer,
    hentet: Vec<DetaljMelding>,
) -> Markup {
    let rader = vis_meldinger(&funn.meldinger, hentet);
    let finnes = || rader.iter().filter(|r| r.kilde.is_some());
    let ulik_person = ulike(finnes().map(|r| r.person.clone()));
    let ulik_type = ulike(finnes().map(|r| r.utfoert_av_type));
    let ulik_av = ulike(finnes().map(|r| r.utfoert_av.clone()));
    let ulikt_tidspunkt =
        ulike(finnes().map(|r| r.kilde.as_ref().and_then(|m| m.utfoert_tidspunkt_ms)));
    html! {
        p { a href="/stats/?fane=funn" { "Tilbake til funn" } }
        h1 { "Funndetaljer" }
        dl.funn-detaljer {
            dt { "Regel" } dd { (regel.navn()) " (versjon " (versjon) ")" }
            dt { "Hva regelen sjekker" } dd { (regel.beskrivelse()) }
            dt { "Utfall" } dd { (kode_tekst(regel, funn.kode)) }
            dt { "Alvorlighet" } dd { (alvorlighet_merke(funn.alvorlighet)) }
            @for (navn, verdi) in funn.detaljer.iter() {
                dt { (navn) } dd { (verdi) }
            }
        }
        h2 { "Meldinger" }
        p.dempet {
            "Første melding er den som sist bestemte funnet. Personer vises som bokstaver "
            "som bare gjelder på denne siden."
        }
        div.tabell { table {
            thead { tr {
                th { "Topic" } th.tall { "Partisjon" } th.tall { "Offset" }
                th { "Tidspunkt (Kafka)" } th { "Meldingstype" } th { "Kilde" }
                (kolonne("Person", ulik_person))
                (kolonne("Utført av-type", ulik_type))
                (kolonne("Utført av", ulik_av))
                (kolonne("Utført tidspunkt", ulikt_tidspunkt))
                th { "Trace-ID" }
            } }
            tbody { @for rad in &rader {
                tr {
                    @if let Some(m) = &rad.kilde {
                        td { (m.topic) }
                        td.tall { (m.record_partition) }
                        td.tall { (m.record_offset) }
                        td { (tidspunkt_tekst(m.record_timestamp_ms)) }
                        td { (m.meldingstype.as_deref().unwrap_or(STREK)) }
                        td { (m.kilde.as_deref().unwrap_or(STREK)) }
                        td { (rad.person.as_deref().unwrap_or(STREK)) }
                        td { (rad.utfoert_av_type.map_or(STREK, utfoert_av_type_tekst)) }
                        td { (rad.utfoert_av.as_deref().unwrap_or(STREK)) }
                        td { (tidspunkt_tekst(m.utfoert_tidspunkt_ms)) }
                        td.mono { (trace_id_tekst(m.trace_id)) }
                    } @else {
                        td colspan="11" { "Melding " (rad.id) " finnes ikke lenger." }
                    }
                }
            } }
        } }
    }
}
