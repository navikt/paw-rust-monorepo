#[derive(thiserror::Error, Debug, Eq, PartialEq)]
pub enum AvroSerdeError {
    #[error("Failed to serialize Avro message: {0}")]
    AvroSerializationFailed(String),
    #[error("Failed to deserialize Avro message: {0}")]
    AvroDeserializationFailed(String),
    #[error("Avro deserialization error: {0}")]
    AvroError(String),
}

impl From<apache_avro::Error> for AvroSerdeError {
    fn from(e: apache_avro::Error) -> Self {
        Self::AvroError(e.to_string())
    }
}
