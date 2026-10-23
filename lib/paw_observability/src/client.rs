use opentelemetry::global;
use opentelemetry_http::HeaderInjector;
use reqwest::header::HeaderMap;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub fn inject_trace_context_header() -> HeaderMap {
    let context = tracing::Span::current().context();
    let mut headers = HeaderMap::new();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut HeaderInjector(&mut headers))
    });
    headers
}
