use crate::model::error::validation_error::ValidationError;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::{DateTime, Utc};
use paw_sqlx::error::DatabaseError;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ProblemDetails {
    #[serde(rename = "type")]
    pub(crate) problem_type: String,
    pub(crate) title: String,
    pub(crate) status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) detail: Option<String>,
    pub(crate) instance: String,
    pub(crate) timestamp: DateTime<Utc>,
}

impl ProblemDetails {
    pub(crate) fn validation_error(instance: String, error: ValidationError) -> Self {
        Self {
            problem_type: "urn:paw:http:validation-error".to_string(),
            title: "Bad Request".to_string(),
            status: 400u16,
            detail: Some(error.to_string()),
            instance,
            timestamp: Utc::now(),
        }
    }
    pub(crate) fn internal_server_error(instance: String) -> Self {
        Self {
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
