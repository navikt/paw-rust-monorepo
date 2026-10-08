use super::format::OtelFormat;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct OtelTracingConfig {
    pub format: OtelFormat,
    pub directives: Vec<String>,
}

pub fn default_config() -> OtelTracingConfig {
    OtelTracingConfig {
        format: OtelFormat::OtelJson,
        directives: [
            "debug",
            "sqlx::query=info",
            // h2/tonic/hyper/rustls bærer OTLP-gRPC-transporten for span-eksport.
            // På DEBUG-nivå logger h2 én terse linje ("received"/"send") per
            // HTTP/2-frame, som drukner ut nyttige applikasjonslogger.
            "h2=info",
            "tonic=info",
            "tower=info",
            "hyper=info",
            "hyper_util=info",
            "rustls=info",
            // opentelemetry sine interne otel_debug!/otel_info!-kall (aktivert av
            // "internal-logs", som er default-feature) har ikke noe "message"-felt,
            // kun et strukturert "name"-felt. På DEBUG-niva ser disse ut som tomme
            // logglinjer i Grafana/Loki, som viser "message" som forhandsvisning.
            "opentelemetry=info",
            "opentelemetry_sdk=info",
            "opentelemetry-otlp=info",
        ]
        .map(String::from)
        .to_vec(),
    }
}
