use super::json_format::OtelJsonFormat;
use serde::Deserialize;
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OtelFormat {
    Json,
    OtelJson,
    Full,
    Pretty,
    #[serde(other)]
    #[default]
    Compact,
}

impl<S, N> FormatEvent<S, N> for OtelFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        match self {
            OtelFormat::Json => tracing_subscriber::fmt::format()
                .json()
                .format_event(ctx, writer, event),
            OtelFormat::OtelJson => OtelJsonFormat.format_event(ctx, writer, event),
            OtelFormat::Full => tracing_subscriber::fmt::format().format_event(ctx, writer, event),
            OtelFormat::Pretty => tracing_subscriber::fmt::format()
                .pretty()
                .format_event(ctx, writer, event),
            OtelFormat::Compact => tracing_subscriber::fmt::format()
                .compact()
                .format_event(ctx, writer, event),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OtelFormat;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct Buffer(Arc<Mutex<Vec<u8>>>);

    impl Write for Buffer {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn otel_json_gir_gyldig_json_for_verdier_med_sitattegn_og_linjeskift() {
        let buffer = Buffer::default();
        let writer = buffer.clone();
        let subscriber = tracing_subscriber::fmt()
            .event_format(OtelFormat::OtelJson)
            .with_writer(move || writer.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(felt = "a\"b\nc", debug_felt = ?"x\"y", "melding med \"sitat\"\nog linjeskift");
        });

        let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        let line = output.lines().next().unwrap();
        let json: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(json["felt"], "a\"b\nc");
        assert_eq!(json["debug_felt"], "\"x\\\"y\"");
        assert_eq!(json["message"], "melding med \"sitat\"\nog linjeskift");
    }
}
