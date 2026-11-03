use apache_avro::error::Details;
use apache_avro::types::{Value, ValueKind};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum AvroSerdeError {
    #[error("Ugyldig Avro-skjema: {kind}")]
    InvalidSchema { kind: String },
    #[error("Kunne ikke serialisere Avro-melding: {kind}")]
    Serialize { kind: String },
    #[error("Kunne ikke deserialisere Avro-melding: {kind}")]
    Deserialize { kind: String },
    #[error("Avro-melding har {remaining} bytes igjen etter dekoding")]
    TrailingBytes { remaining: usize },
    #[error("Ukjent enum: {0}")]
    UkjentEnumType(String),
}

impl AvroSerdeError {
    pub(crate) fn invalid_schema(e: apache_avro::Error) -> Self {
        Self::InvalidSchema { kind: kind(&e) }
    }

    pub(crate) fn serialize(e: apache_avro::Error) -> Self {
        Self::Serialize { kind: kind(&e) }
    }

    pub(crate) fn deserialize(e: apache_avro::Error) -> Self {
        Self::deserialize_with_context(e, "convert to Rust")
    }

    pub(crate) fn reader(e: apache_avro::Error) -> Self {
        Self::deserialize_with_context(e, "create reader")
    }

    pub(crate) fn decode(e: apache_avro::Error, consumed: u64, length: usize) -> Self {
        Self::deserialize_with_context(
            e,
            &format!("decode (consumed {consumed} of {length} bytes)"),
        )
    }

    fn deserialize_with_context(e: apache_avro::Error, context: &str) -> Self {
        Self::Deserialize {
            kind: format!("{context}: {}", kind(&e)),
        }
    }
}

fn kind(e: &apache_avro::Error) -> String {
    let description = match e.details() {
        Details::ConvertI64ToUsize(..) => "Cannot convert i64 to usize",
        Details::ConvertI32ToUsize(..) => "Cannot convert i32 to usize",
        Details::ConvertU64ToUsize(..) => "Cannot convert u64 to usize",
        Details::ConvertI64ToU64(..) => "Cannot convert i64 to u64",
        Details::ConvertI32ToU64(..) => "Cannot convert i32 to u64",
        Details::ConvertI64ToU128(..) => "Cannot convert i64 to u128",
        Details::ConvertI32ToU128(..) => "Cannot convert i32 to u128",
        Details::ConvertUsizeToI64(..) => "Cannot convert usize to i64",
        Details::ConvertLengthToI32(..) => "Cannot convert length to i32",
        Details::ZagI32(..) => "Decoded integer out of range for i32",
        Details::IntegerOverflow => "Overflow when decoding integer",
        Details::GetDate(value) => return expected_type("Value::Date or Value::Int", value),
        Details::GetTimeMillis(value) => {
            return expected_type("Value::TimeMillis or Value::Int", value);
        }
        Details::GetTimeMicros(value) => {
            return expected_type("Value::TimeMicros, Value::Long or Value::Int", value);
        }
        Details::GetTimestampMillis(value) => {
            return expected_type("Value::TimestampMillis, Value::Long or Value::Int", value);
        }
        Details::GetTimestampMicros(value) => {
            return expected_type("Value::TimestampMicros, Value::Long or Value::Int", value);
        }
        Details::GetTimestampNanos(value) => {
            return expected_type("Value::TimestampNanos, Value::Long or Value::Int", value);
        }
        Details::GetLocalTimestampMillis(value) => {
            return expected_type(
                "Value::LocalTimestampMillis, Value::Long or Value::Int",
                value,
            );
        }
        Details::GetLocalTimestampMicros(value) => {
            return expected_type(
                "Value::LocalTimestampMicros, Value::Long or Value::Int",
                value,
            );
        }
        Details::GetLocalTimestampNanos(value) => {
            return expected_type(
                "Value::LocalTimestampNanos, Value::Long or Value::Int",
                value,
            );
        }
        Details::GetNull(value) => return expected_type("Value::Null", value),
        Details::GetBoolean(value) => return expected_type("Value::Boolean", value),
        Details::GetInt(value) => return expected_type("Value::Int", value),
        Details::GetLong(value) => return expected_type("Value::Long or Value::Int", value),
        Details::GetFloat(value) => {
            return expected_type(
                "Value::Float, Value::Double, Value::Int, Value::Long or Value::String",
                value,
            );
        }
        Details::GetDouble(value) => {
            return expected_type(
                "Value::Double, Value::Float, Value::Int, Value::Long or Value::String",
                value,
            );
        }
        Details::GetString(value) => {
            return expected_type("Value::String, Value::Bytes or Value::Fixed", value);
        }
        Details::GetBytes(value) | Details::BytesValue(value) => {
            return expected_type("Value::Bytes", value);
        }
        Details::FixedValue(value) => return expected_type("Value::Fixed", value),
        Details::GetEnum(value) => return expected_type("Value::Enum", value),
        Details::GetUuid(value) => return expected_type("Value::Uuid", value),
        Details::GetBigDecimal(value) => return expected_type("Value::BigDecimal", value),
        Details::GetArray { other, .. } => return expected_type("Value::Array", other),
        Details::GetMap { other, .. } => return expected_type("Value::Map", other),
        Details::GetRecord { other, .. } => return expected_type("Value::Record", other),
        Details::GetU8(value) => {
            return format!("Cannot convert Value::{:?} to u8", ValueKind::from(value));
        }
        Details::ReadVariableIntegerBytes(source) => {
            return io_kind("Failed to read variable length integer", source);
        }
        Details::ReadString(source) => return io_kind("Failed to read string", source),
        Details::ReadBytes(source) => return io_kind("Failed to read bytes", source),
        Details::ReadBoolean(source) => return io_kind("Failed to read boolean", source),
        Details::ReadDouble(source) => return io_kind("Failed to read double", source),
        Details::ReadFloat(source) => return io_kind("Failed to read float", source),
        Details::ReadDuration(source) => return io_kind("Failed to read duration", source),
        Details::ReadFixed(source, _) => return io_kind("Failed to read fixed", source),
        Details::ConvertToUtf8(..) | Details::ConvertToUtf8Error(..) => "Invalid UTF-8 string",
        Details::BoolValue(..) => "Invalid boolean encoding",
        Details::GetUnionVariant { .. } => "Union index out of bounds",
        Details::EnumSymbolIndex { .. } | Details::GetEnumValue { .. } => {
            "Enum symbol index out of bounds"
        }
        Details::GetEnumSymbol(..) => "Enum symbol not found",
        Details::GetEnumUnknownIndexValue => "Unable to decode enum index",
        Details::FindUnionVariant { .. } => "No matching union variant",
        Details::GetField(..) => "Missing field in record",
        Details::MemoryAllocation { .. } => "Avro allocation limit exceeded",
        Details::CompareFixedSizes { .. } => "Fixed size mismatch",
        Details::Validation | Details::ValidationWithReason { .. } => "Value does not match schema",
        Details::DeserializeValue(..) | Details::DeserializeSchemaAware { .. } => {
            "Failed to deserialize Avro value into Rust value"
        }
        Details::SerializeValue(..) | Details::SerializeValueWithSchema { .. } => {
            "Failed to serialize Rust value into Avro value"
        }
        Details::SerializeRecordFieldWithSchema { .. } => "Failed to serialize record field",
        Details::MissingDefaultForSkippedField { .. } => "Missing default for skipped field",
        Details::ParseSchemaJson(..) => "Failed to parse schema JSON",
        Details::ParsePrimitive(..) | Details::ParsePrimitiveSimilar(..) => {
            "Unknown primitive schema type"
        }
        Details::SchemaResolutionError(..) | Details::NoEntryInLookupTable(..) => {
            "Unresolved schema reference"
        }
        Details::NameCollision(..) | Details::AmbiguousSchemaDefinition(..) => {
            "Ambiguous schema definition"
        }
        // Unmapped details can contain values, schema defaults or arbitrary error text.
        _ => "Avro error (details omitted)",
    };
    description.to_string()
}

fn expected_type(expected: &str, actual: &Value) -> String {
    format!(
        "Expected {expected}, got Value::{:?}",
        ValueKind::from(actual)
    )
}

fn io_kind(description: &str, source: &std::io::Error) -> String {
    format!("{description}: {:?}", source.kind())
}

pub fn enum_type_not_found(type_: &str) -> AvroSerdeError {
    AvroSerdeError::UkjentEnumType(type_.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::error::Details;
    use apache_avro::types::Value;

    #[test]
    fn kind_inneholder_ikke_verdier() {
        for mapper in [
            AvroSerdeError::deserialize,
            AvroSerdeError::serialize,
            AvroSerdeError::invalid_schema,
        ] {
            for details in [
                Details::GetDate(Value::String("12345678901".into())),
                Details::GetField("Ola Nordmann".into()),
                Details::DeserializeValue("invalid type: string \"Ola Nordmann\"".into()),
                Details::DeserializeValue("Ola Nordmann".into()),
                Details::SerializeValue("Storgata".into()),
                Details::ParsePrimitive("Ola Nordmann".into()),
                Details::GetInt(Value::Record(vec![(
                    "Ola Nordmann".into(),
                    Value::String("12345678901".into()),
                )])),
                Details::ReadString(std::io::Error::other("Ola Nordmann")),
                Details::InvalidMetadataKey("Ola Nordmann".into()),
            ] {
                let err = mapper(details.into());
                let melding = format!("{err} {err:?}");
                assert!(!melding.contains("12345678901"), "{melding}");
                assert!(!melding.contains("Ola"), "{melding}");
                assert!(!melding.contains("Storgata"), "{melding}");
                assert!(std::error::Error::source(&err).is_none());
            }
        }
    }

    #[test]
    fn kind_beholder_trygg_feilbeskrivelse() {
        let err = AvroSerdeError::deserialize(Details::GetField("x".into()).into());
        assert_eq!(
            err,
            AvroSerdeError::Deserialize {
                kind: "convert to Rust: Missing field in record".to_string()
            }
        );
    }

    #[test]
    fn kind_beholder_hele_typeparet_uten_tallverdien() {
        let source = usize::try_from(-123_456_789_012_i64).unwrap_err();
        let err = AvroSerdeError::deserialize(
            Details::ConvertI64ToUsize(source, -123_456_789_012).into(),
        );
        assert_eq!(
            err,
            AvroSerdeError::Deserialize {
                kind: "convert to Rust: Cannot convert i64 to usize".into(),
            }
        );
    }

    #[test]
    fn heltallskonverteringer_beholder_hele_typeparet() {
        let source = u64::try_from(-1_i64).unwrap_err();
        for (details, expected) in [
            (Details::ConvertI64ToUsize(source, -1), "i64 to usize"),
            (Details::ConvertI32ToUsize(source, -1), "i32 to usize"),
            (Details::ConvertU64ToUsize(source, u64::MAX), "u64 to usize"),
            (Details::ConvertI64ToU64(source, -1), "i64 to u64"),
            (Details::ConvertI32ToU64(source, -1), "i32 to u64"),
            (Details::ConvertI64ToU128(source, -1), "i64 to u128"),
            (Details::ConvertI32ToU128(source, -1), "i32 to u128"),
            (
                Details::ConvertUsizeToI64(source, usize::MAX),
                "usize to i64",
            ),
            (
                Details::ConvertLengthToI32(source, usize::MAX),
                "length to i32",
            ),
        ] {
            assert_eq!(kind(&details.into()), format!("Cannot convert {expected}"));
        }
    }

    #[test]
    fn ukartlagt_feil_faar_trygg_beskrivelse() {
        assert_eq!(
            kind(&Details::InvalidMetadataKey("Ola Nordmann".into()).into()),
            "Avro error (details omitted)"
        );
    }

    #[test]
    fn reader_feil_viser_feilsteg() {
        let err = AvroSerdeError::reader(Details::GetField("Ola Nordmann".into()).into());
        assert_eq!(
            err.to_string(),
            "Kunne ikke deserialisere Avro-melding: create reader: Missing field in record"
        );
    }

    #[test]
    fn typefeil_viser_forventet_og_faktisk_type_uten_verdi() {
        let err = AvroSerdeError::deserialize(
            Details::GetDate(Value::String("12345678901".into())).into(),
        );
        assert_eq!(
            err,
            AvroSerdeError::Deserialize {
                kind: "convert to Rust: Expected Value::Date or Value::Int, got Value::String"
                    .into(),
            }
        );
    }
}
