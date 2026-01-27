use crate::error_handling::AppError;
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

impl AppError for EnvVarNotFoundError {
    fn error_name(&self) -> &'static str {
        "EnvVarNotFoundError"
    }
}

pub fn get_env(var: &'static str) -> Result<String, EnvVarNotFoundError> {
    let key = var;
    std::env::var(key).map_err(|_| EnvVarNotFoundError { env_var_name: var })
}
