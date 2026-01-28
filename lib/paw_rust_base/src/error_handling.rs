use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub enum ErrorType {
    InternalError,
    InputValidationError,
    TemporaryError,
    AuthenticationError,
    AuthorizationError,
}

pub trait AppError: Error + Debug + Display + 'static {
    fn error_name(&self) -> &'static str;
    fn error_message(&self) -> String;
    fn error_type(&self) -> ErrorType;
}

pub struct GenericAppError {}

impl AppError for GenericAppError {
    fn error_name(&self) -> &'static str {
        "GenericAppError"
    }
    fn error_message(&self) -> String {
        "A generic application error occurred".to_string()
    }
    fn error_type(&self) -> ErrorType {
        ErrorType::InternalError
    }
}

impl Debug for GenericAppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.error_name()))
    }
}

impl Display for GenericAppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.error_name()))
    }
}

impl Error for GenericAppError {}
