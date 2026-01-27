use axum::extract::MatchedPath;
use axum::http::{HeaderMap, Request};
use axum::Router;
use opentelemetry::propagation::Extractor;
use opentelemetry::trace::Status;
use opentelemetry::Context;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::trace::TraceLayer;
use tracing::{info_span, warn, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

struct HeaderExtractor<'a>(&'a HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

pub fn extract_trace_context(headers: &HeaderMap) -> Context {
    let extractor = HeaderExtractor(headers);
    opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&extractor))
}

pub fn add_otel_trace_layer<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router.layer(
        TraceLayer::new_for_http()
            .make_span_with(|request: &Request<axum::body::Body>| {
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
            })
            .on_response(
                |response: &axum::response::Response,
                 _latency: std::time::Duration,
                 span: &Span| {
                    let code = response.status().as_u16();
                    if code >= 500 {
                        span.set_status(Status::Error {
                            description: "Internal Server Error".into(),
                        });
                    }
                    span.record("http.response.status_code", response.status().as_u16());
                },
            )
            .on_failure(
                |error: ServerErrorsFailureClass, _latency: std::time::Duration, span: &Span| {
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
                },
            ),
    )
}
