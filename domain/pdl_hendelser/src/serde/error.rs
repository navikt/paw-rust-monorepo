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
        Self::Deserialize { kind: kind(&e) }
    }
}

fn kind(e: &apache_avro::Error) -> String {
    e.to_string()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == ' ')
        .take(100)
        .collect::<String>()
        .trim_end()
        .to_string()
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
        for details in [
            Details::GetDate(Value::String("12345678901".into())),
            Details::GetField("Ola Nordmann".into()),
            Details::DeserializeValue("invalid type: string \"Ola Nordmann\"".into()),
        ] {
            let err = AvroSerdeError::deserialize(details.into());
            let melding = format!("{err} {err:?}");
            assert!(!melding.contains("12345678901"), "{melding}");
            assert!(!melding.contains("Ola"), "{melding}");
        }
    }

    #[test]
    fn kind_beholder_innledende_tekst() {
        let err = AvroSerdeError::deserialize(Details::GetField("x".into()).into());
        assert_eq!(
            err,
            AvroSerdeError::Deserialize {
                kind: "Missing field in record".to_string()
            }
        );
    }
}
