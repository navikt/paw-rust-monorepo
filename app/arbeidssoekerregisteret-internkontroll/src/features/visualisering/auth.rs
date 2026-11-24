use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use oauth2::principal::Principal;
use paw_oauth2_resource_server::oauth2::{config::AuthConfig, state::AuthState};
use paw_rust_base::config::read_toml_config;

/// Auth for visualiseringen. `None` bare når appen er bygget med featuren
/// `local`: alle andre bygg krever innlogging, også om miljøet oppdages feil.
pub async fn auth_state(auth_config_toml: &str) -> Result<Option<Arc<AuthState>>, Box<dyn Error>> {
    if cfg!(feature = "local") {
        tracing::warn!("Bygget med feature local: visualiseringen kjører uten auth");
        return Ok(None);
    }
    let config: AuthConfig = read_toml_config(auth_config_toml)?;
    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let state = AuthState::new(config, http_client)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(state))
}

/// Slipper bare gjennom innloggede Nav-ansatte. Maskin-til-maskin-tokens
/// godtas av OAuth2-laget, men skal ikke se visualiseringen.
pub async fn krev_nav_ansatt(request: Request, next: Next) -> Response {
    match request.extensions().get::<Principal>() {
        Some(Principal::NavAnsatt(_)) => next.run(request).await,
        _ => StatusCode::FORBIDDEN.into_response(),
    }
}
