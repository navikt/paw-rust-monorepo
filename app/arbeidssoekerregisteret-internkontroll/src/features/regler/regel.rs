use std::pin::Pin;

use paw_kafka::topics::{Topic, get_topic};
use paw_rust_base::env::RuntimeEnv;
use sqlx::PgConnection;

use crate::features::innlesing::topic_ids::TopicIds;
use crate::features::regler::hele_trace_signert::HeleTraceSignert;
use crate::features::regler::ingen_periode_uten_startet::IngenPeriodeUtenStartet;
use crate::features::regler::periode_startet_lik_startet::PeriodeStartetLikStartet;
use crate::features::regler::startet_er_signert::StartetErSignert;
use crate::features::regler::startet_gir_periode::StartetGirPeriode;
use crate::features::regler::{
    batch::BatchRad, funn::Vurdering, nivaa::Nivaa, oppslag::Sett, regel_katalog::RegelId,
    regelversjon::Regelversjon,
};

pub type VurderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Vurdering, sqlx::Error>> + Send + 'a>>;

pub trait Regel: Send + Sync {
    fn id(&self) -> RegelId;
    fn versjon(&self) -> Regelversjon;
    /// Hva regelen vurderer. Bestemmer hvilke funn en vurdering erstatter.
    fn nivaa(&self) -> Nivaa;
    /// Topics regelen leser. Alle må ligge i topic-katalogen.
    fn topics(&self) -> &'static [Topic];

    /// Vurderer én melding. `sett` er fremdriften før `rad`, én per topic i
    /// [`Regel::topics`]. Historikk som slås opp, skal begrenses til
    /// `record_offset <= siste_offset` for sitt topic: det er meldingene regelen
    /// allerede har behandlet.
    fn vurder<'a>(
        &'a self,
        conn: &'a mut PgConnection,
        rad: &'a BatchRad,
        sett: &'a Sett<'a>,
    ) -> VurderFuture<'a>;
}

/// En regel med topic-ID-ene slått opp. Lages én gang ved oppstart.
pub struct RegistrertRegel {
    pub regel: Box<dyn Regel>,
    pub topic_ids: Vec<i16>,
    /// I samme rekkefølge som `regel.topics()`.
    pub topics: Vec<(Topic, i16)>,
}

pub fn registrer_regler(
    regler: Vec<Box<dyn Regel>>,
    topic_ids: &TopicIds,
    runtime_env: &RuntimeEnv,
) -> Vec<RegistrertRegel> {
    regler
        .into_iter()
        .map(|regel| {
            let topic_ids: Vec<i16> = regel
                .topics()
                .iter()
                .map(|topic| {
                    let navn = get_topic(runtime_env, topic);
                    topic_ids
                        .get(navn)
                        .unwrap_or_else(|| panic!("Topic {navn} mangler i topic-katalogen"))
                })
                .collect();
            let topics = regel
                .topics()
                .iter()
                .cloned()
                .zip(topic_ids.iter().copied())
                .collect();
            RegistrertRegel {
                regel,
                topic_ids,
                topics,
            }
        })
        .collect()
}

pub fn alle_regler() -> Vec<Box<dyn Regel>> {
    vec![
        Box::new(HeleTraceSignert),
        Box::new(StartetErSignert),
        Box::new(IngenPeriodeUtenStartet),
        Box::new(StartetGirPeriode),
        Box::new(PeriodeStartetLikStartet),
    ]
}
