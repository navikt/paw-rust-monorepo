mod common;

use arbeidssoekerregisteret_internkontroll::features::regler::hele_trace_signert::HeleTraceSignert;
use common::{TestApp, melding::Melding, topic_navn};
use paw_kafka::topics::Topic;
use paw_test::hendelse_builder::StartetBuilder;
use rdkafka::Message;
use uuid::Uuid;

fn traceparent(trace: Uuid) -> String {
    format!("00-{}-b7ad6b7169203331-01", trace.simple())
}

async fn les_inn(app: &TestApp, trace: Option<Uuid>, signert: bool) {
    let melding = Melding::ny(StartetBuilder::default());
    let melding = match trace {
        Some(trace) => melding.header("traceparent", traceparent(trace)),
        None => melding,
    };
    let msg = app.les_inn(melding).await;
    if !signert {
        sqlx::query(
            "UPDATE record_metadata SET signature_status = 'missing_signature'
             WHERE record_offset = $1",
        )
        .bind(msg.offset())
        .execute(&app.pool)
        .await
        .unwrap();
    }
}

fn hendelselogg(offset: i64) -> (String, i64, i16) {
    (topic_navn(&Topic::Hendelselogg).to_string(), offset, 1)
}

#[tokio::test]
async fn trace_med_usignert_melding_gir_ett_funn_paa_siste_usignerte() {
    let app = TestApp::start().await;
    let feil = Uuid::new_v4();
    let ok = Uuid::new_v4();
    les_inn(&app, Some(feil), true).await;
    les_inn(&app, Some(feil), false).await;
    les_inn(&app, Some(ok), true).await;
    les_inn(&app, Some(feil), false).await;
    les_inn(&app, Some(feil), true).await;
    les_inn(&app, None, false).await;

    app.kjor_regler(vec![Box::new(HeleTraceSignert)]).await;

    assert_eq!(app.funn().await, [hendelselogg(3)]);
}
