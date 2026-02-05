use crate::error_handling::{AppError, ErrorType};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct EnvVarNotFoundError {
    pub env_var_name: &'static str,
}

impl Error for EnvVarNotFoundError {}

impl Debug for EnvVarNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "EnvVarNotFoundError {{ env_var_name: {} }}",
            self.env_var_name
        ))
    }
}

impl Display for EnvVarNotFoundError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "Environment variable '{}' not found",
            self.env_var_name
        ))
    }
}

impl From<EnvVarNotFoundError> for Box<dyn AppError> {
    fn from(value: EnvVarNotFoundError) -> Self {
        Box::new(value)
    }
}

impl AppError for EnvVarNotFoundError {
    fn error_name(&self) -> &'static str {
        "EnvVarNotFoundError"
    }

    fn error_message(&self) -> String {
        format!("Environment variable '{}' not found", self.env_var_name)
    }

    fn error_type(&self) -> ErrorType {
        ErrorType::InternalError
    }
}

pub fn get_env(var: &'static str) -> Result<String, EnvVarNotFoundError> {
    std::env::var(var).map_err(|_| EnvVarNotFoundError { env_var_name: var })
}
