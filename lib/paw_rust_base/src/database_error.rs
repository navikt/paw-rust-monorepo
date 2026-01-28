use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use crate::error_handling::{AppError, ErrorType};

pub struct DatabaseError {
    pub message: String,
}

impl Error for DatabaseError {}

impl Debug for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DatabaseError: {}", self.message)
    }
}

impl Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Database error: {}", self.message)
    }
}

impl AppError for DatabaseError {
    fn error_name(&self) -> &'static str {
        "DatabaseError"
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }

    fn error_type(&self) -> ErrorType {
        ErrorType::InternalError
    }
}