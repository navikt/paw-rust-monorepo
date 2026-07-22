use paw_key_gen_client::model::IdentitetType;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use std::error::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DaoError {
    #[error(
        "Fant ingen rader for ID i {table}-tabellen for melding på topic '{topic}', partition {partition}, offset {offset}"
    )]
    NoRows {
        table: String,
        topic: String,
        partition: i32,
        offset: i64,
    },
    #[error(
        "Fant flere rader ({count}) for ID i {table}-tabellen for melding på topic '{topic}', partition {partition}, offset {offset}"
    )]
    MultipleRows {
        table: String,
        count: usize,
        topic: String,
        partition: i32,
        offset: i64,
    },
}

impl DaoError {
    pub fn no_rows(owned_message: &OwnedMessage, table: &str) -> Self {
        DaoError::NoRows {
            table: table.to_string(),
            topic: owned_message.topic().to_string(),
            partition: owned_message.partition(),
            offset: owned_message.offset(),
        }
    }

    pub fn multiple_rows(owned_message: &OwnedMessage, table: &str, count: usize) -> Self {
        DaoError::MultipleRows {
            table: table.to_string(),
            count,
            topic: owned_message.topic().to_string(),
            partition: owned_message.partition(),
            offset: owned_message.offset(),
        }
    }
}

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error(
        "Fant ingen gjeldende {identity_type}-identitet for message from topic '{topic}' at partition {partition}, offset {offset}"
    )]
    NotFound {
        identity_type: String,
        topic: String,
        partition: i32,
        offset: i64,
    },
}

impl IdentityError {
    pub fn not_found(owned_message: &OwnedMessage, identitet_type: IdentitetType) -> Self {
        IdentityError::NotFound {
            identity_type: identitet_type.as_ref().to_string(),
            topic: owned_message.topic().to_string(),
            partition: owned_message.partition(),
            offset: owned_message.offset(),
        }
    }
}

#[derive(Error, Debug)]
pub enum PayloadProcessorError {
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

impl PayloadProcessorError {
    pub fn deserialization_error(owned_message: &OwnedMessage, error: &dyn Error) -> Self {
        PayloadProcessorError::DeserializationError {
            message: error.to_string(),
            topic: owned_message.topic().to_string(),
            partition: owned_message.partition(),
            offset: owned_message.offset(),
        }
    }

    pub fn no_payload_error(owned_message: &OwnedMessage) -> Self {
        PayloadProcessorError::NoPayload {
            topic: owned_message.topic().to_string(),
            partition: owned_message.partition(),
            offset: owned_message.offset(),
        }
    }

    pub fn processing_error(owned_message: &OwnedMessage, message: &str) -> Self {
        PayloadProcessorError::ProcessingError {
            message: message.to_string(),
            topic: owned_message.topic().to_string(),
            partition: owned_message.partition(),
            offset: owned_message.offset(),
        }
    }
}
