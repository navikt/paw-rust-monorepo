use opentelemetry::global;
use opentelemetry_http::HeaderInjector;
use reqwest::header::HeaderMap;
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub fn trace_context_aware_headers() -> HeaderMap {
    let context = tracing::Span::current().context();
    let mut headers = HeaderMap::new();
    global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&context, &mut HeaderInjector(&mut headers))
    });
    headers
}
