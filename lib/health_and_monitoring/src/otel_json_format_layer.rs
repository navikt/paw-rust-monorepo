use chrono;
use opentelemetry::trace::TraceContextExt;
use paw_rust_base::git;
use std::fmt::Write as FmtWrite;
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct OtelJsonFormat;

/// Serialiserer en verdi som en JSON-streng, inkludert omsluttende anførselstegn.
/// Brukes for alle tekstverdier som skrives inn i det håndrullede JSON-formatet,
/// slik at anførselstegn, backslash og kontrolltegn i verdien (f.eks. fra
/// `{:?}`-formatterte struct-felter) ikke ødelegger JSON-strukturen på loggselen.
fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

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
        write!(
            &mut writer,
            "\"timestamp\":{}",
            json_string(&now.to_rfc3339())
        )?;

        // Add level
        write!(
            &mut writer,
            ",\"log_level\":{}",
            json_string(&meta.level().to_string())
        )?;

        // Add target
        write!(&mut writer, ",\"target\":{}", json_string(meta.target()))?;

        write!(
            &mut writer,
            ",\"git_sha\":{}",
            json_string(git::commit_hash())
        )?;

        // Add file and line
        if let Some(file) = meta.file() {
            write!(&mut writer, ",\"file\":{}", json_string(file))?;
            //Tar med logger_name slik at rust apper logger med samme format som andre språk,
            //blir enklere å kjøre felles søk i loki.
            let logger_name = file.strip_suffix(".rs").unwrap_or(file).replace("/", ".");
            write!(
                &mut writer,
                ",\"logger_name\":{}",
                json_string(&logger_name)
            )?;
        }
        if let Some(line) = meta.line() {
            write!(&mut writer, ",\"line\":{}", line)?;
        }

        let otel_context = opentelemetry::Context::current();
        let otel_span = otel_context.span();
        let span_context = otel_span.span_context();

        if span_context.is_valid() {
            write!(
                &mut writer,
                ",\"trace_id\":{}",
                json_string(&span_context.trace_id().to_string())
            )?;
            write!(
                &mut writer,
                ",\"span_id\":{}",
                json_string(&span_context.span_id().to_string())
            )?;
        }

        if let Some(span) = ctx.lookup_current() {
            write!(&mut writer, ",\"span\":{}", json_string(span.name()))?;
        }

        struct FieldVisitor<W> {
            writer: W,
            result: std::fmt::Result,
        }

        impl<W: FmtWrite> tracing::field::Visit for FieldVisitor<W> {
            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if self.result.is_err() {
                    return;
                }
                self.result = write!(
                    &mut self.writer,
                    ",\"{}\":{}",
                    field.name(),
                    json_string(value)
                );
            }

            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if self.result.is_err() {
                    return;
                }
                let formatted = format!("{:?}", value);
                self.result = write!(
                    &mut self.writer,
                    ",\"{}\":{}",
                    field.name(),
                    json_string(&formatted)
                );
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

