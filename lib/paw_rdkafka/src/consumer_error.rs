use paw_rust_base::error_handling::{AppError, ErrorType};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub struct ConsumerError {
    pub message: String,
}

impl Error for ConsumerError {}

impl Debug for ConsumerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DatabaseError: {}", self.message)
    }
}

impl Display for ConsumerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Consumer error: {}", self.message)
    }
}
impl From<ConsumerError> for Box<dyn AppError> {
    fn from(value: ConsumerError) -> Self {
        Box::new(value)
    }
}

impl AppError for ConsumerError {
    fn error_name(&self) -> &'static str {
        "ConsumerError"
    }

    fn error_message(&self) -> String {
        self.message.clone()
    }

    fn error_type(&self) -> ErrorType {
        ErrorType::InternalError
    }
}
