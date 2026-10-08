//! HTTP-servermetrikker som speiler Ktor `MicrometerMetrics` med `percentilesHistogram(true)`.
//!
//! Eksponerer ett histogram, `http_server_requests_seconds`, med labelene `method`, `route` og `status`.
//! Bucket-grensene er de samme som Micrometer genererer for Kotlin-appene våre
//! (min 20 ms, maks 1 s, SLO 150 ms og 500 ms), slik at samme Grafana-spørringer kan brukes.
//!
//! ```ignore
//! Router::new()
//!     .route("/api/v1/noe", get(handler))
//!     .route_layer(paw_observability::http_metrics::http_metrics_layer())
//! ```

use axum::extract::MatchedPath;
use axum::http::{Method, Request, Response};
use pin_project_lite::pin_project;
use prometheus::{HistogramOpts, HistogramVec, register_histogram_vec};
use std::future::Future;
use std::pin::Pin;
use std::sync::LazyLock;
use std::task::{Context, Poll, ready};
use std::time::Instant;
use tower::{Layer, Service};

pub const HTTP_SERVER_REQUESTS_SECONDS: &str = "http_server_requests_seconds";

/// Micrometer `PercentileHistogramBuckets` filtrert til [20 ms, 1 s], pluss SLO 150 ms og 500 ms.
pub const BUCKETS: &[f64] = &[
    0.022369621,
    0.027962026,
    0.033554431,
    0.039146836,
    0.044739241,
    0.050331646,
    0.055924051,
    0.061516456,
    0.067108864,
    0.089478485,
    0.111848106,
    0.134217727,
    0.15,
    0.156587348,
    0.178956969,
    0.20132659,
    0.223696211,
    0.246065832,
    0.268435456,
    0.357913941,
    0.447392426,
    0.5,
    0.536870911,
    0.626349396,
    0.715827881,
    0.805306366,
    0.894784851,
    0.984263336,
];

const NOT_AVAILABLE: &str = "n/a";

static HTTP_SERVER_REQUESTS: LazyLock<HistogramVec> = LazyLock::new(|| {
    register_histogram_vec!(
        HistogramOpts::new(
            HTTP_SERVER_REQUESTS_SECONDS,
            "Varighet på HTTP-forespørsler"
        )
        .buckets(BUCKETS.to_vec()),
        &["method", "route", "status"]
    )
    .expect("http_server_requests_seconds skal kun registreres én gang")
});

/// Bruk med `Router::route_layer` (kun matchede ruter) eller `Router::layer`
/// (umatchede forespørsler får `route="n/a"`).
pub fn http_metrics_layer() -> HttpMetricsLayer {
    HttpMetricsLayer
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpMetricsLayer;

impl<S> Layer<S> for HttpMetricsLayer {
    type Service = HttpMetrics<S>;

    fn layer(&self, inner: S) -> Self::Service {
        HttpMetrics { inner }
    }
}

#[derive(Clone, Debug)]
pub struct HttpMetrics<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for HttpMetrics<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let method = method_label(req.method());
        let route = req.extensions().get::<MatchedPath>().cloned();
        ResponseFuture {
            inner: self.inner.call(req),
            start: Instant::now(),
            method,
            route,
        }
    }
}

pin_project! {
    pub struct ResponseFuture<F> {
        #[pin]
        inner: F,
        start: Instant,
        method: &'static str,
        route: Option<MatchedPath>,
    }
}

impl<F, ResBody, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<ResBody>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = ready!(this.inner.poll(cx));
        let status_code = result.as_ref().ok().map(Response::status);
        let status = status_code.as_ref().map_or(NOT_AVAILABLE, |s| s.as_str());
        let route = this
            .route
            .as_ref()
            .map_or(NOT_AVAILABLE, MatchedPath::as_str);
        HTTP_SERVER_REQUESTS
            .with_label_values(&[*this.method, route, status])
            .observe(this.start.elapsed().as_secs_f64());
        Poll::Ready(result)
    }
}

/// Fast sett med verdier: klienten styrer metoden, og ukjente metoder skal ikke gi nye tidsserier.
fn method_label(method: &Method) -> &'static str {
    match method.as_str() {
        "GET" => "GET",
        "POST" => "POST",
        "PUT" => "PUT",
        "PATCH" => "PATCH",
        "DELETE" => "DELETE",
        "HEAD" => "HEAD",
        "OPTIONS" => "OPTIONS",
        _ => "OTHER",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::StatusCode;
    use axum::{Router, routing::get};
    use tower::ServiceExt;

    fn count(method: &str, route: &str, status: &str) -> u64 {
        HTTP_SERVER_REQUESTS
            .with_label_values(&[method, route, status])
            .get_sample_count()
    }

    #[tokio::test]
    async fn registrerer_route_mal_og_status() {
        let app = Router::new()
            .route("/test/sak/{id}", get(|| async { StatusCode::CREATED }))
            .route_layer(http_metrics_layer());
        let before = count("GET", "/test/sak/{id}", "201");

        let req = Request::get("/test/sak/42").body(Body::empty()).unwrap();
        let res = app.oneshot(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(count("GET", "/test/sak/{id}", "201"), before + 1);
    }

    #[tokio::test]
    async fn umatchet_rute_gir_na_med_layer() {
        let app = Router::new()
            .route("/test/finnes", get(|| async { "ok" }))
            .layer(http_metrics_layer());
        let before = count("GET", "n/a", "404");

        let req = Request::get("/test/finnes-ikke/123")
            .body(Body::empty())
            .unwrap();
        let res = app.oneshot(req).await.unwrap();

        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        assert_eq!(count("GET", "n/a", "404"), before + 1);
    }

    #[test]
    fn ukjent_metode_gir_other() {
        let method = Method::from_bytes(b"FOOBAR").unwrap();
        assert_eq!(method_label(&method), "OTHER");
    }
}
