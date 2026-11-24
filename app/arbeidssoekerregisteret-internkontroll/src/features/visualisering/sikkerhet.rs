use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::Response,
};
use tower_http::set_header::SetResponseHeaderLayer;

/// HX-Redirect ved 401 og sikkerhetsheadere. Ligger utenfor auth-laget, så
/// også avviste svar får headerne.
pub fn sikkerhetslag<S: Clone + Send + Sync + 'static>(router: Router<S>) -> Router<S> {
    router
        .layer(middleware::from_fn(hx_redirect_ved_401))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static("default-src 'self'; frame-ancestors 'none'"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("no-referrer"),
        ))
}

/// Når sesjonen er utløpt, laster HTMX hele siden på nytt så Wonderwall
/// kan logge inn igjen. Fast sti, aldri en verdi fra requesten.
async fn hx_redirect_ved_401(request: Request, next: Next) -> Response {
    let er_htmx = request
        .headers()
        .get("HX-Request")
        .is_some_and(|v| v == "true");
    let mut response = next.run(request).await;
    if er_htmx && response.status() == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert("HX-Redirect", HeaderValue::from_static("/stats/"));
    }
    response
}
