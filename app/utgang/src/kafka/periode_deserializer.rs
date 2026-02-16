use apache_avro::types::Value as AvroValue;
use chrono::{DateTime, Utc};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum PeriodeDeserializerError {
    #[error("Failed to deserialize Avro message: {0}")]
    AvroDeserializationFailed(String),
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

        // Convert Avro value to Periode struct
        let periode = self.avro_to_periode(&decoded.value)?;

        Ok(periode)
    }

    /// Convert Avro Value to Periode struct
    fn avro_to_periode(&self, avro_value: &AvroValue) -> Result<Periode, PeriodeDeserializerError> {
        match avro_value {
            AvroValue::Record(fields) => {
                let id = self.extract_uuid(fields, "id")?;
                let identitetsnummer = self.extract_string(fields, "identitetsnummer")?;
                let startet = self.extract_metadata(fields, "startet")?;
                let avsluttet = self.extract_optional_metadata(fields, "avsluttet")?;

                Ok(Periode {
                    id,
                    identitetsnummer,
                    startet,
                    avsluttet,
                })
            }
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Expected Avro Record for Periode, got: {:?}", avro_value)
            )),
        }
    }

    fn extract_metadata(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<Metadata, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Field '{}' not found", field_name)
            ))?;

        match value {
            AvroValue::Record(metadata_fields) => {
                let tidspunkt = self.extract_timestamp_millis(metadata_fields, "tidspunkt")?;
                let utfoert_av = self.extract_bruker(metadata_fields, "utfoertAv")?;
                let kilde = self.extract_string(metadata_fields, "kilde")?;
                let aarsak = self.extract_string(metadata_fields, "aarsak")?;
                let tidspunkt_fra_kilde = self.extract_optional_tidspunkt_fra_kilde(metadata_fields, "tidspunktFraKilde")?;

                Ok(Metadata {
                    tidspunkt,
                    utfoert_av,
                    kilde,
                    aarsak,
                    tidspunkt_fra_kilde,
                })
            }
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(
                "Expected Avro Record for Metadata".to_string()
            )),
        }
    }

    fn extract_optional_metadata(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<Option<Metadata>, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Field '{}' not found", field_name)
            ))?;

        match value {
            AvroValue::Union(_, boxed_value) => match boxed_value.as_ref() {
                AvroValue::Null => Ok(None),
                AvroValue::Record(metadata_fields) => {
                    let tidspunkt = self.extract_timestamp_millis(metadata_fields, "tidspunkt")?;
                    let utfoert_av = self.extract_bruker(metadata_fields, "utfoertAv")?;
                    let kilde = self.extract_string(metadata_fields, "kilde")?;
                    let aarsak = self.extract_string(metadata_fields, "aarsak")?;
                    let tidspunkt_fra_kilde = self.extract_optional_tidspunkt_fra_kilde(metadata_fields, "tidspunktFraKilde")?;

                    Ok(Some(Metadata {
                        tidspunkt,
                        utfoert_av,
                        kilde,
                        aarsak,
                        tidspunkt_fra_kilde,
                    }))
                }
                _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(
                    "Expected Record or Null in Union for optional Metadata".to_string()
                )),
            },
            AvroValue::Null => Ok(None),
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(
                "Expected Union or Null for optional Metadata field".to_string()
            )),
        }
    }

    fn extract_bruker(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<Bruker, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(format!("Field '{}' not found", field_name)))?;

        match value {
            AvroValue::Record(bruker_fields) => {
                let bruker_type = self.extract_bruker_type(bruker_fields, "type")?;
                let id = self.extract_string(bruker_fields, "id")?;
                let sikkerhetsnivaa = self.extract_optional_string(bruker_fields, "sikkerhetsnivaa")?;

                Ok(Bruker {
                    bruker_type,
                    id,
                    sikkerhetsnivaa,
                })
            }
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed("Expected Avro Record for Bruker".to_string())),
        }
    }

    fn extract_bruker_type(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<BrukerType, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(format!("Field '{}' not found", field_name)))?;

        match value {
            AvroValue::Enum(_, symbol) => match symbol.as_str() {
                "UKJENT_VERDI" => Ok(BrukerType::UkjentVerdi),
                "UDEFINERT" => Ok(BrukerType::Udefinert),
                "VEILEDER" => Ok(BrukerType::Veileder),
                "SYSTEM" => Ok(BrukerType::System),
                "SLUTTBRUKER" => Ok(BrukerType::Sluttbruker),
                _ => Ok(BrukerType::UkjentVerdi),
            },
            _ => Ok(BrukerType::UkjentVerdi),
        }
    }

    fn extract_optional_tidspunkt_fra_kilde(
        &self,
        fields: &[(String, AvroValue)],
        field_name: &str,
    ) -> Result<Option<TidspunktFraKilde>, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value);

        match value {
            None => Ok(None),
            Some(AvroValue::Union(_, boxed_value)) => match boxed_value.as_ref() {
                AvroValue::Null => Ok(None),
                AvroValue::Record(tfk_fields) => {
                    let tidspunkt = self.extract_timestamp_millis(tfk_fields, "tidspunkt")?;
                    let avviks_type = self.extract_avviks_type(tfk_fields, "avviksType")?;

                    Ok(Some(TidspunktFraKilde {
                        tidspunkt,
                        avviks_type,
                    }))
                }
                _ => Err(PeriodeDeserializerError::AvroDeserializationFailed("Expected Record or Null in Union for TidspunktFraKilde".to_string())),
            },
            Some(AvroValue::Null) => Ok(None),
            Some(_) => Err(PeriodeDeserializerError::AvroDeserializationFailed("Expected Union or Null for optional TidspunktFraKilde".to_string())),
        }
    }

    fn extract_avviks_type(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<AvviksType, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(format!("Field '{}' not found", field_name)))?;

        match value {
            AvroValue::Enum(_, symbol) => match symbol.as_str() {
                "UKJENT_VERDI" => Ok(AvviksType::UkjentVerdi),
                "FORSINKELSE" => Ok(AvviksType::Forsinkelse),
                "RETTING" => Ok(AvviksType::Retting),
                "SLETTET" => Ok(AvviksType::Slettet),
                "TIDSPUNKT_KORRIGERT" => Ok(AvviksType::TidspunktKorrigert),
                _ => Ok(AvviksType::UkjentVerdi),
            },
            _ => Ok(AvviksType::UkjentVerdi),
        }
    }

    fn extract_uuid(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<Uuid, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Field '{}' not found", field_name)
            ))?;

        match value {
            AvroValue::Uuid(uuid) => Ok(*uuid),
            AvroValue::String(s) => {
                Uuid::parse_str(s).map_err(|_| PeriodeDeserializerError::AvroDeserializationFailed(
                    format!("Invalid UUID string: {}", s)
                ))
            }
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Expected UUID or String for field '{}'", field_name)
            )),
        }
    }

    fn extract_string(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<String, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(format!("Field '{}' not found", field_name)))?;

        match value {
            AvroValue::String(s) => Ok(s.clone()),
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(format!("Expected String for field '{}'", field_name))),
        }
    }

    fn extract_optional_string(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<Option<String>, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value);

        match value {
            None => Ok(None),
            Some(AvroValue::Union(_, boxed_value)) => match boxed_value.as_ref() {
                AvroValue::Null => Ok(None),
                AvroValue::String(s) => Ok(Some(s.clone())),
                _ => Err(PeriodeDeserializerError::AvroDeserializationFailed("Expected String or Null in Union for optional String field".to_string())),
            },
            Some(AvroValue::Null) => Ok(None),
            Some(AvroValue::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(PeriodeDeserializerError::AvroDeserializationFailed("Expected Union, Null, or String for optional String field".to_string())),
        }
    }

    fn extract_timestamp_millis(&self, fields: &[(String, AvroValue)], field_name: &str) -> Result<DateTime<Utc>, PeriodeDeserializerError> {
        let value = fields
            .iter()
            .find(|(name, _)| name == field_name)
            .map(|(_, value)| value)
            .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Field '{}' not found", field_name)
            ))?;

        match value {
            AvroValue::TimestampMillis(millis) => {
                DateTime::from_timestamp_millis(*millis)
                    .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(
                        format!("Invalid timestamp millis: {}", millis)
                    ))
            }
            AvroValue::Long(millis) => {
                DateTime::from_timestamp_millis(*millis)
                    .ok_or_else(|| PeriodeDeserializerError::AvroDeserializationFailed(
                        format!("Invalid timestamp millis: {}", millis)
                    ))
            }
            _ => Err(PeriodeDeserializerError::AvroDeserializationFailed(
                format!("Expected TimestampMillis or Long for field '{}'", field_name)
            )),
        }
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

