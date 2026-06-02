use thiserror::Error;

#[derive(Error, Debug)]
pub enum HendelseProcessorError {
    #[error(
        "Failed to deserialize payload from topic '{topic}' at partition {partition}, offset {offset}: {message}"
    )]
    DeserializationError {
        message: String,
        topic: String,
        partition: i32,
        offset: i64,
    },
    #[error(
        "Message has no payload from topic '{topic}' at partition {partition}, offset {offset}"
    )]
    NoPayload {
        topic: String,
        partition: i32,
        offset: i64,
    },
    #[error(
        "Processing failed for message from topic '{topic}' at partition {partition}, offset {offset}: {message}"
    )]
    ProcessingError {
        message: String,
        topic: String,
        partition: i32,
        offset: i64,
    },
}
