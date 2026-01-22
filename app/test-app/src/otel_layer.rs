use chrono;
use opentelemetry::trace::TraceContextExt;
use std::fmt::Write as FmtWrite;
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct OtelJsonFormat;

impl<S, N> FormatEvent<S, N> for OtelJsonFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let meta = event.metadata();

        // Start JSON object
        write!(&mut writer, "{{")?;

        // Add timestamp
        let now = chrono::Utc::now();
        write!(&mut writer, "\"timestamp\":\"{}\"", now.to_rfc3339())?;

        // Add level
        write!(&mut writer, ",\"level\":\"{}\"", meta.level())?;

        // Add target
        write!(&mut writer, ",\"target\":\"{}\"", meta.target())?;

        // Add file and line
        if let Some(file) = meta.file() {
            write!(&mut writer, ",\"file\":\"{}\"", file)?;
        }
        if let Some(line) = meta.line() {
            write!(&mut writer, ",\"line\":{}", line)?;
        }

        let otel_context = opentelemetry::Context::current();
        let otel_span = otel_context.span();
        let span_context = otel_span.span_context();

        if span_context.is_valid() {
            write!(&mut writer, ",\"trace_id\":\"{}\"", span_context.trace_id())?;
            write!(&mut writer, ",\"span_id\":\"{}\"", span_context.span_id())?;
        }

        if let Some(span) = ctx.lookup_current() {
            write!(&mut writer, ",\"span\":\"{}\"", span.name())?;
        }

        struct FieldVisitor<W> {
            writer: W,
            result: std::fmt::Result,
        }

        impl<W: FmtWrite> tracing::field::Visit for FieldVisitor<W> {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if self.result.is_err() {
                    return;
                }
                self.result = write!(&mut self.writer, ",\"{}\":\"{:?}\"", field.name(), value);
            }

            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if self.result.is_err() {
                    return;
                }
                self.result = write!(&mut self.writer, ",\"{}\":\"{}\"", field.name(), value);
            }
        }

        let mut visitor = FieldVisitor {
            writer: &mut writer,
            result: Ok(()),
        };
        event.record(&mut visitor);
        visitor.result?;

        write!(&mut writer, "}}")?;

        writeln!(&mut writer)
    }
}
