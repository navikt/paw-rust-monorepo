use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
use paw_oauth2_resource_server::oauth2::{
    oidc::JwksCache,
    state::{AuthState, IssuerState},
};
use tower::ServiceExt;

use super::{VisState, routes, sikkerhet::sikkerhetslag};

const NOKKEL: &str = "test-hemmelighet-som-bare-brukes-i-tester";
const UTSTEDER: &str = "https://issuer.invalid";
const KLIENT: &str = "internkontroll";

/// Alle ruter i visualiseringen. Ny rute skal legges til her, slik at testene
/// under sjekker at den er beskyttet.
const RUTER: &[&str] = &[
    "/stats",
    "/stats/",
    "/stats/?fane=funn",
    "/stats/toppfelt",
    "/stats/static/htmx.min.js",
    "/stats/static/app.css",
];

fn state() -> VisState {
    VisState {
        pg_pool: sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://unused:unused@localhost/unused")
            .unwrap(),
        hwm_version: 1,
    }
}

fn auth_state() -> Arc<AuthState> {
    Arc::new(AuthState {
        azure: Some(IssuerState {
            cache: tokio::sync::RwLock::new(JwksCache {
                keys: HashMap::from([(
                    "test-key".to_owned(),
                    DecodingKey::from_secret(NOKKEL.as_bytes()),
                )]),
                fetched_at: Instant::now(),
            }),
            well_known_url: "http://unused.invalid".to_owned(),
            expected_issuer: UTSTEDER.to_owned(),
            client_id: KLIENT.to_owned(),
            http_client: reqwest::Client::new(),
        }),
        tokenx: None,
        idporten: None,
        maskinporten: None,
    })
}

fn token(claims: serde_json::Value) -> String {
    let header = Header {
        kid: Some("test-key".to_owned()),
        ..Header::default()
    };
    let mut claims = claims;
    claims["iss"] = UTSTEDER.into();
    claims["aud"] = KLIENT.into();
    claims["exp"] = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600)
        .into();
    encode(
        &header,
        &claims,
        &EncodingKey::from_secret(NOKKEL.as_bytes()),
    )
    .unwrap()
}

fn nav_ansatt_token() -> String {
    token(serde_json::json!({ "oid": "ansatt-oid", "NAVident": "Z999999" }))
}

fn system_token() -> String {
    token(serde_json::json!({ "oid": "system-oid", "roles": ["access_as_application"] }))
}

async fn get_med(app: &Router, sti: &str, token: Option<&str>) -> axum::response::Response {
    let mut req = Request::get(sti);
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn alle_ruter_krever_token() {
    let app = routes(state(), Some(auth_state()));
    for sti in RUTER {
        for token in [None, Some("ugyldig")] {
            let response = get_med(&app, sti, token).await;
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "{sti} {token:?}"
            );
            assert_eq!(response.headers()[header::X_FRAME_OPTIONS], "DENY", "{sti}");
        }
    }
}

#[tokio::test]
async fn systemtoken_avvises() {
    let app = routes(state(), Some(auth_state()));
    let token = system_token();
    for sti in RUTER {
        let response = get_med(&app, sti, Some(&token)).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{sti}");
    }
}

#[tokio::test]
async fn nav_ansatt_slipper_inn() {
    let app = routes(state(), Some(auth_state()));
    let token = nav_ansatt_token();
    for sti in ["/stats/static/htmx.min.js", "/stats/static/app.css"] {
        let response = get_med(&app, sti, Some(&token)).await;
        assert_eq!(response.status(), StatusCode::OK, "{sti}");
    }
}

#[tokio::test]
async fn sikkerhetsheadere_settes() {
    let app = routes(state(), None);
    let response = get_med(&app, "/stats/static/app.css", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let h = response.headers();
    assert_eq!(h[header::X_FRAME_OPTIONS], "DENY");
    assert_eq!(h[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(h[header::CACHE_CONTROL], "no-store");
    let csp = h[header::CONTENT_SECURITY_POLICY].to_str().unwrap();
    assert!(csp.starts_with("default-src 'self'"));
    assert!(!csp.contains("unsafe-inline"));
}

#[tokio::test]
async fn ugyldige_valg_avvises_foer_databasen() {
    let app = routes(state(), None);
    for sti in [
        "/stats/?fane=ukjent",
        "/stats/?fane=funn&regel=999",
        "/stats/?fane=funn&regel=abc",
        "/stats/?fane=funn&alvorlighet=alvorlig",
    ] {
        let response = get_med(&app, sti, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{sti}");
    }
}

#[tokio::test]
async fn hx_redirect_bare_for_htmx_ved_401() {
    let app = sikkerhetslag(Router::new().route("/x", get(|| async { StatusCode::UNAUTHORIZED })));
    let kall = |htmx: bool| {
        let mut req = Request::get("/x");
        if htmx {
            req = req.header("HX-Request", "true");
        }
        app.clone().oneshot(req.body(Body::empty()).unwrap())
    };
    let med = kall(true).await.unwrap();
    assert_eq!(med.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(med.headers()["HX-Redirect"], "/stats/");
    assert!(
        !kall(false)
            .await
            .unwrap()
            .headers()
            .contains_key("HX-Redirect")
    );
}
