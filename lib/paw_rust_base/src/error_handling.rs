use std::error::Error;
use std::fmt::{Debug, Display, Formatter};

pub trait AppError: Error + Debug + Display + 'static {
    fn error_name(&self) -> &'static str;
}

pub struct GenericAppError {}

impl AppError for GenericAppError {
    fn error_name(&self) -> &'static str {
        "GenericAppError"
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
