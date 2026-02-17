use chrono::{DateTime, Utc};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum PeriodeDeserializerError {
    #[error("Failed to deserialize Avro message: {0}")]
    AvroDeserializationFailed(String),
    #[error("Avro deserialization error: {0}")]
    AvroError(#[from] apache_avro::Error),
}

/// Represents a user/actor in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bruker {
    #[serde(rename = "type")]
    pub bruker_type: BrukerType,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sikkerhetsnivaa: Option<String>,
}

/// Type of user/actor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BrukerType {
    UkjentVerdi,
    Udefinert,
    Veileder,
    System,
    Sluttbruker,
}

/// Type of time deviation between source and registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AvviksType {
    UkjentVerdi,
    Forsinkelse,
    #[deprecated(note = "Use SLETTET instead")]
    Retting,
    Slettet,
    TidspunktKorrigert,
}

/// Information about time deviation from source
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TidspunktFraKilde {
    pub tidspunkt: DateTime<Utc>,
    pub avviks_type: AvviksType,
}

/// Metadata about a change in the job seeker registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    /// Timestamp of the change
    pub tidspunkt: DateTime<Utc>,
    /// Who performed the change
    pub utfoert_av: Bruker,
    /// Name of the system that performed the change
    pub kilde: String,
    /// Reason for the change
    pub aarsak: String,
    /// Time deviation from source, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tidspunkt_fra_kilde: Option<TidspunktFraKilde>,
}

/// Represents a period where a user has been registered as a job seeker
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Periode {
    /// Unique identifier for the period
    pub id: Uuid,
    /// Identity number (fødselsnummer or d-nummer) of the person
    pub identitetsnummer: String,
    /// Information about when the period started and who started it
    pub startet: Metadata,
    /// Information about when the period ended and who ended it.
    /// None if the period is still ongoing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avsluttet: Option<Metadata>,
}

impl Periode {
    /// Check if the period is currently active (not ended)
    pub fn is_active(&self) -> bool {
        self.avsluttet.is_none()
    }
}

/// Deserializer for Periode messages from Kafka using Schema Registry
pub struct PeriodeDeserializer {
    decoder: schema_registry_converter::async_impl::avro::AvroDecoder<'static>,
}

impl PeriodeDeserializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let decoder = schema_registry_converter::async_impl::avro::AvroDecoder::new(schema_reg_settings);
        Self { decoder }
    }

    pub async fn deserialize(&self, payload: &[u8]) -> Result<Periode, PeriodeDeserializerError> {
        // Decode using schema registry
        let decoded = self
            .decoder
            .decode(Some(payload))
            .await
            .map_err(|e| PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Failed to decode Avro message with schema registry: {}", e)
            ))?;

        // Convert Avro value to Periode struct using apache_avro's built-in serde support
        let periode: Periode = apache_avro::from_value(&decoded.value)?;

        Ok(periode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_periode_is_active() {
        let active_periode = Periode {
            id: Uuid::new_v4(),
            identitetsnummer: "12345678901".to_string(),
            startet: create_test_metadata(),
            avsluttet: None,
        };
        assert!(active_periode.is_active());

        let ended_periode = Periode {
            id: Uuid::new_v4(),
            identitetsnummer: "12345678901".to_string(),
            startet: create_test_metadata(),
            avsluttet: Some(create_test_metadata()),
        };
        assert!(!ended_periode.is_active());
    }

    fn create_test_metadata() -> Metadata {
        Metadata {
            tidspunkt: Utc::now(),
            utfoert_av: Bruker {
                bruker_type: BrukerType::Sluttbruker,
                id: "12345678901".to_string(),
                sikkerhetsnivaa: Some("tokenx:Level4".to_string()),
            },
            kilde: "test-system".to_string(),
            aarsak: "Test".to_string(),
            tidspunkt_fra_kilde: None,
        }
    }
}

