use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use errors::auth::AuthError;
use errors::database::DatabaseError;
use errors::validation::ValidationError;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, ToSchema)]
pub struct ProblemDetails {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub problem_type: String,
    pub title: String,
    pub status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub instance: String,
    pub timestamp: DateTime<Utc>,
}

impl ProblemDetails {
    pub fn validation_error(instance: String, detail: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            problem_type: "urn:paw:http:validation-error".to_string(),
            title: "Bad Request".to_string(),
            status: 400u16,
            detail: Some(detail),
            instance,
            timestamp: Utc::now(),
        }
    }
    pub fn unauthorized(instance: String, error: AuthError) -> Self {
        Self {
            id: Uuid::new_v4(),
            problem_type: "urn:paw:http:unauthorized".to_string(),
            title: "Unauthorized".to_string(),
            status: 401u16,
            detail: Some(error.to_string()),
            instance,
            timestamp: Utc::now(),
        }
    }

    pub fn internal_server_error(instance: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            problem_type: "urn:paw:default:unhandled-error".to_string(),
            title: "Internal Server Error".to_string(),
            status: 500u16,
            detail: None,
            instance,
            timestamp: Utc::now(),
        }
    }
}

impl IntoResponse for ProblemDetails {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let mut response = (status, Json(self)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

impl From<ValidationError> for ProblemDetails {
    fn from(e: ValidationError) -> Self {
        tracing::warn!(error = %e, "Validering feilet");
        Self::validation_error("/".to_string(), e.to_string())
    }
}

impl From<AuthError> for ProblemDetails {
    fn from(e: AuthError) -> Self {
        tracing::warn!(error = %e, "Autentisering feilet");
        Self::unauthorized("/".to_string(), e)
    }
}

impl From<DatabaseError> for ProblemDetails {
    fn from(e: DatabaseError) -> Self {
        tracing::error!(error = %e, "Spørring mot database feilet");
        Self::internal_server_error("/".to_string())
    }
}

impl From<anyhow::Error> for ProblemDetails {
    fn from(e: anyhow::Error) -> Self {
        tracing::error!(error = %e, "Det oppsto en uhåndtert feil");
        Self::internal_server_error("/".to_string())
    }
}
