trait AppError {
    fn error_name() -> &'static str;
}

pub struct GenericAppError {}

impl AppError for GenericAppError {
    fn error_name() -> &'static str {
        "GenericAppError"
    }
}
