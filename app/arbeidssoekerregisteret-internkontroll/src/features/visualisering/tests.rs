use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, encode};
use paw_oauth2_resource_server::oauth2::{
    oidc::JwksCache,
    state::{AuthState, IssuerState},
};
use tower::ServiceExt;

use super::{VisState, routes};

const NOKKEL: &str = "test-hemmelighet-som-bare-brukes-i-tester";
const UTSTEDER: &str = "https://issuer.invalid";
const KLIENT: &str = "internkontroll";

/// Alle ruter i visualiseringen. Ny rute skal legges til her, slik at testene
/// under sjekker at den er beskyttet.
const RUTER: &[&str] = &[
    "/stats",
    "/stats/",
    "/stats/?fane=funn",
    "/stats/funn?regel=5&versjon=2&melding=1",
    "/stats/fragment/fane?fane=trace",
    "/stats/fragment/toppfelt",
    "/stats/static/oppdater.js",
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
    for sti in ["/stats/static/oppdater.js", "/stats/static/app.css"] {
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
    assert!(csp.starts_with("default-src 'none'"));
    assert!(csp.contains("script-src 'self'"));
    assert!(!csp.contains("unsafe"));
}

#[tokio::test]
async fn ugyldige_valg_avvises_foer_databasen() {
    let app = routes(state(), None);
    for sti in [
        "/stats/?fane=ukjent",
        "/stats/?fane=funn&regel=999",
        "/stats/?fane=funn&regel=abc",
        "/stats/?fane=funn&alvorlighet=alvorlig",
        "/stats/fragment/fane?fane=ukjent",
        "/stats/fragment/fane?fane=funn&regel=999",
    ] {
        let response = get_med(&app, sti, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{sti}");
    }
}

async fn post_tema(app: &Router, body: &str, token: Option<&str>) -> axum::response::Response {
    let mut req = Request::post("/stats/tema")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
    if let Some(token) = token {
        req = req.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    app.clone()
        .oneshot(req.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn tema_krever_nav_ansatt() {
    let app = routes(state(), Some(auth_state()));
    assert_eq!(
        post_tema(&app, "tema=mork", None).await.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        post_tema(&app, "tema=mork", Some(&system_token()))
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let response = post_tema(&app, "tema=mork", Some(&nav_ansatt_token())).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn tema_settes_i_cookie_og_sender_tilbake() {
    let app = routes(state(), None);
    for tema in ["lys", "mork", "auto"] {
        let response = post_tema(&app, &format!("tema={tema}"), None).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{tema}");
        assert_eq!(response.headers()[header::LOCATION], "/stats/");
        assert_eq!(
            response.headers()[header::SET_COOKIE],
            format!("tema={tema}; Path=/stats; Max-Age=31536000; SameSite=Lax; HttpOnly; Secure")
        );
    }
}

#[tokio::test]
async fn ugyldig_tema_avvises() {
    let app = routes(state(), None);
    for body in [
        "tema=ukjent",
        "tema=",
        "",
        "tema=lys%3B%20Domain%3Devil",
        "annet=lys",
    ] {
        let response = post_tema(&app, body, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{body:?}");
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }
}
