use std::{error::Error, fmt::Display};

use paw_rust_base::error_handling::AppError;

#[derive(Debug)]
pub struct TexasClientError {
    pub texas_response_code: u16,
    pub target: String,
    pub message: String,
}

impl AppError for TexasClientError {
    fn error_name(&self) -> &'static str {
        "TexasClientError"
    }

    fn error_message(&self) -> String {
        format!(
            "Failed to get token for target {}: {}",
            self.target, self.message
        )
    }

    fn error_type(&self) -> paw_rust_base::error_handling::ErrorType {
        match self.texas_response_code {
            401 => paw_rust_base::error_handling::ErrorType::AuthenticationError,
            403 => paw_rust_base::error_handling::ErrorType::AuthorizationError,
            503 => paw_rust_base::error_handling::ErrorType::TemporaryError,
            _ => paw_rust_base::error_handling::ErrorType::InternalError,
        }
    }
}

impl Error for TexasClientError {}

impl From<TexasClientError> for Box<dyn AppError> {
    fn from(error: TexasClientError) -> Self {
        Box::new(error)
    }
}

impl Display for TexasClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to get token for target {}: {}",
            self.target, self.message
        )
    }
}
