use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

pub struct AppError {
    pub(crate) domain: String,
    pub(crate) value: String,
}

impl Debug for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} :: {}", self.domain, self.value)
    }
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} :: {}", self.domain, self.value)
    }
}


impl Error for AppError {}

pub fn get_env(var: &str) -> Result<String, AppError> {
    let key = var;
    std::env::var(key).map_err(|_| AppError {
        domain: "get_env_var".to_string(),
        value: format!("Failed to get env var {}", var),
    })
}
