use crate::header_extractor::extract_trace_context;
use axum::extract::MatchedPath;
use axum::http::Request;
use opentelemetry::trace::Status;
use std::time::Duration;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::{
    DefaultOnBodyChunk, DefaultOnEos, DefaultOnRequest, HttpMakeClassifier, TraceLayer,
};
use tracing::{info_span, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

pub type OtelTraceLayer = TraceLayer<
    HttpMakeClassifier,
    fn(&Request<axum::body::Body>) -> Span,
    DefaultOnRequest,
    fn(&axum::response::Response<axum::body::Body>, Duration, &Span),
    DefaultOnBodyChunk,
    DefaultOnEos,
    fn(ServerErrorsFailureClass, Duration, &Span),
>;

pub fn otel_middleware() -> OtelTraceLayer {
    TraceLayer::new_for_http()
        .make_span_with(make_span as fn(&Request<axum::body::Body>) -> Span)
        .on_response(
            record_response as fn(&axum::response::Response<axum::body::Body>, Duration, &Span),
        )
        .on_failure(record_failure as fn(ServerErrorsFailureClass, Duration, &Span))
}

fn make_span(request: &Request<axum::body::Body>) -> Span {
    let matched_path = request
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str);
    let parent_ctx = extract_trace_context(request.headers());
    let span = info_span!(
        "http_request",
        http.request.method = ?request.method(),
        http.route = matched_path,
        some_other_field = tracing::field::Empty,
    );
    let _res = span.set_parent(parent_ctx);
    span
}

fn record_response(
    response: &axum::response::Response<axum::body::Body>,
    _latency: Duration,
    span: &Span,
) {
    let code = response.status().as_u16();
    if code >= 500 {
        span.set_status(Status::Error {
            description: "Internal Server Error".into(),
        });
    }
    span.record("http.response.status_code", response.status().as_u16());
}

fn record_failure(error: ServerErrorsFailureClass, _latency: Duration, span: &Span) {
    let backtrace = std::backtrace::Backtrace::capture();
    match error {
        ServerErrorsFailureClass::Error(_err) => {
            warn!("HTTP server error class: Error");
        }
        ServerErrorsFailureClass::StatusCode(code) => {
            warn!("HTTP server error class: StatusCode {}", code);
        }
    }
    span.record("stack_trace", format!("{}", backtrace).as_str());
    span.set_status(Status::Error {
        description: "Internal Server Error".into(),
    });
}
