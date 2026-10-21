use chrono::Utc;
use paw_error_handling::problem_details::ProblemDetails;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TilgangError {
    #[error("Kunne ikke tolke felt '{field}'")]
    ExtractFieldFailed { field: String, instance: String },
}

impl TilgangError {
    pub fn nav_ident(instance: String) -> Self {
        Self::ExtractFieldFailed {
            field: "nav_ident".to_string(),
            instance,
        }
    }

    pub fn identitetsnummer(instance: String) -> Self {
        Self::ExtractFieldFailed {
            field: "identitetsnummer".to_string(),
            instance,
        }
    }

    pub fn kontor_id(instance: String) -> Self {
        Self::ExtractFieldFailed {
            field: "kontor_id".to_string(),
            instance,
        }
    }

    pub fn instance(&self) -> String {
        match self {
            TilgangError::ExtractFieldFailed { field: _, instance } => instance.clone(),
        }
    }
}

impl From<TilgangError> for ProblemDetails {
    fn from(e: TilgangError) -> Self {
        Self {
            id: Uuid::new_v4(),
            problem_type: "".to_string(),
            title: "".to_string(),
            status: 400,
            detail: None,
            instance: e.instance(),
            timestamp: Utc::now(),
        }
    }
}
