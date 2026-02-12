use paw_rust_base::error_handling::{AppError, ErrorType};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct ConfigError {
    pub message: String,
}

impl Error for ConfigError {}

impl Debug for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConfigError: {}", self.message)
    }
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Config error: {}", self.message)
    }
}

impl From<ConfigError> for Box<dyn AppError> {
    fn from(value: ConfigError) -> Self {
        Box::new(value)
    }
}

impl AppError for ConfigError {
    fn error_name(&self) -> &'static str {
        "ConfigError"
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }

    fn error_type(&self) -> ErrorType {
        ErrorType::InternalError
    }
}
