use crate::serde::error::AvroSerdeError;
use apache_avro::Schema;
use apache_avro::reader::datum::GenericDatumReader;
use apache_avro::writer::datum::GenericDatumWriter;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::io::Cursor;
use std::sync::Arc;

/// Deserialiserer rå Avro binary datum (uten magic byte, fingerprint eller OCF-header)
/// mot et lokalt skjema.
#[derive(Clone, Debug)]
pub struct AvroDatumDeserializer {
    schema: Arc<Schema>,
}

impl AvroDatumDeserializer {
    pub fn new(schema_json: &str) -> Result<Self, AvroSerdeError> {
        Ok(Self {
            schema: Arc::new(parse_schema(schema_json)?),
        })
    }

    pub fn deserialize<T: DeserializeOwned>(&self, payload: &[u8]) -> Result<T, AvroSerdeError> {
        let (value, remaining) = self.read(payload)?;
        if remaining > 0 {
            return Err(AvroSerdeError::TrailingBytes { remaining });
        }
        apache_avro::from_value(&value).map_err(AvroSerdeError::deserialize)
    }

    fn read(&self, payload: &[u8]) -> Result<(apache_avro::types::Value, usize), AvroSerdeError> {
        let mut cursor = Cursor::new(payload);
        let value = GenericDatumReader::builder(&self.schema)
            .build()
            .and_then(|reader| reader.read_value(&mut cursor))
            .map_err(AvroSerdeError::deserialize)?;
        let remaining = payload.len() - cursor.position() as usize;
        Ok((value, remaining))
    }
}

/// Serialiserer til rå Avro binary datum mot et lokalt skjema.
#[derive(Clone, Debug)]
pub struct AvroDatumSerializer {
    schema: Arc<Schema>,
}

impl AvroDatumSerializer {
    pub fn new(schema_json: &str) -> Result<Self, AvroSerdeError> {
        Ok(Self {
            schema: Arc::new(parse_schema(schema_json)?),
        })
    }

    pub fn serialize(&self, value: &impl Serialize) -> Result<Vec<u8>, AvroSerdeError> {
        let value = apache_avro::to_value(value)
            .and_then(|v| v.resolve(&self.schema))
            .map_err(AvroSerdeError::serialize)?;
        GenericDatumWriter::builder(&self.schema)
            .build()
            .and_then(|writer| writer.write_value_to_vec(value))
            .map_err(AvroSerdeError::serialize)
    }
}

fn parse_schema(schema_json: &str) -> Result<Schema, AvroSerdeError> {
    Schema::parse_str(schema_json).map_err(AvroSerdeError::invalid_schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::avro_date::AvroDate;
    use chrono::{DateTime, NaiveDate, Utc};
    use serde::Deserialize;
    use serde_with::{TimestampMilliSeconds, serde_as};

    const SCHEMA: &str = r#"{
        "type": "record",
        "name": "Test",
        "fields": [
            {"name": "id", "type": "string"},
            {"name": "valgfri", "type": ["null", "string"], "default": null},
            {"name": "status", "type": "string"},
            {"name": "statusEnum", "type": {"type": "enum", "name": "Status", "symbols": ["AKTIV", "INAKTIV"]}},
            {"name": "dato", "type": {"type": "int", "logicalType": "date"}},
            {"name": "valgfriDato", "type": ["null", {"type": "int", "logicalType": "date"}], "default": null},
            {"name": "tidspunkt", "type": {"type": "long", "logicalType": "timestamp-millis"}}
        ]
    }"#;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    enum Status {
        Aktiv,
        Inaktiv,
        #[serde(other)]
        UkjentVerdi,
    }

    #[serde_as]
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Test {
        id: String,
        valgfri: Option<String>,
        status: String,
        status_enum: Status,
        #[serde_as(as = "AvroDate")]
        dato: NaiveDate,
        #[serde_as(as = "Option<AvroDate>")]
        valgfri_dato: Option<NaiveDate>,
        #[serde_as(as = "TimestampMilliSeconds<i64>")]
        tidspunkt: DateTime<Utc>,
    }

    fn test_verdi(valgfri: Option<String>, valgfri_dato: Option<NaiveDate>) -> Test {
        Test {
            id: "abc".to_string(),
            valgfri,
            status: "AKTIV".to_string(),
            status_enum: Status::Inaktiv,
            dato: NaiveDate::from_ymd_opt(1950, 3, 1).unwrap(),
            valgfri_dato,
            tidspunkt: DateTime::from_timestamp_millis(1_761_635_000_123).unwrap(),
        }
    }

    #[test]
    fn round_trip() {
        let ser = AvroDatumSerializer::new(SCHEMA).unwrap();
        let de = AvroDatumDeserializer::new(SCHEMA).unwrap();
        for verdi in [
            test_verdi(None, None),
            test_verdi(
                Some("x".to_string()),
                Some(NaiveDate::from_ymd_opt(2026, 10, 28).unwrap()),
            ),
        ] {
            let bytes = ser.serialize(&verdi).unwrap();
            assert_eq!(de.deserialize::<Test>(&bytes).unwrap(), verdi);
        }
    }

    #[test]
    fn streng_til_enum_gir_ukjent_verdi_ved_ukjent_symbol() {
        #[derive(Debug, Deserialize)]
        struct MedStatusEnum {
            status: Status,
        }

        let ser = AvroDatumSerializer::new(SCHEMA).unwrap();
        let de = AvroDatumDeserializer::new(SCHEMA).unwrap();
        for (status, forventet) in [
            ("AKTIV", Status::Aktiv),
            ("INAKTIV", Status::Inaktiv),
            ("NY_STATUS", Status::UkjentVerdi),
        ] {
            let verdi = Test {
                status: status.to_string(),
                ..test_verdi(None, None)
            };
            let bytes = ser.serialize(&verdi).unwrap();
            let lest: MedStatusEnum = de.deserialize(&bytes).unwrap();
            assert_eq!(lest.status, forventet, "status {status}");
        }
    }

    #[test]
    fn avro_enum_med_ukjent_symbol_feiler_ved_serialisering() {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct MedUkjentSymbol {
            status_enum: &'static str,
        }

        let schema = r#"{
            "type": "record",
            "name": "T",
            "fields": [
                {"name": "statusEnum", "type": {"type": "enum", "name": "Status", "symbols": ["AKTIV", "INAKTIV"]}}
            ]
        }"#;
        let ser = AvroDatumSerializer::new(schema).unwrap();
        assert!(matches!(
            ser.serialize(&MedUkjentSymbol {
                status_enum: "NY_STATUS"
            }),
            Err(AvroSerdeError::Serialize { .. })
        ));
    }

    #[test]
    fn feiler_ved_bytes_igjen_etter_dekoding() {
        let ser = AvroDatumSerializer::new(SCHEMA).unwrap();
        let de = AvroDatumDeserializer::new(SCHEMA).unwrap();
        let mut bytes = ser.serialize(&test_verdi(None, None)).unwrap();
        bytes.extend_from_slice(&[0, 0]);
        assert_eq!(
            de.deserialize::<Test>(&bytes).unwrap_err(),
            AvroSerdeError::TrailingBytes { remaining: 2 }
        );
    }

    #[test]
    fn feiler_ved_confluent_header() {
        let ser = AvroDatumSerializer::new(SCHEMA).unwrap();
        let de = AvroDatumDeserializer::new(SCHEMA).unwrap();
        let mut bytes = vec![0, 0, 0, 0, 42];
        bytes.extend(ser.serialize(&test_verdi(None, None)).unwrap());
        assert!(de.deserialize::<Test>(&bytes).is_err());
    }

    #[test]
    fn feilmelding_inneholder_ikke_verdier() {
        let schema =
            r#"{"type": "record", "name": "T", "fields": [{"name": "ident", "type": "int"}]}"#;
        let ser = AvroDatumSerializer::new(schema).unwrap();

        #[derive(Serialize)]
        struct Feil {
            ident: String,
        }

        let err = ser
            .serialize(&Feil {
                ident: "12345678901".to_string(),
            })
            .unwrap_err();
        assert!(matches!(err, AvroSerdeError::Serialize { .. }));
        assert!(!err.to_string().contains("12345678901"));
        assert!(!format!("{err:?}").contains("12345678901"));
    }

    #[test]
    fn ugyldig_skjema() {
        assert!(matches!(
            AvroDatumDeserializer::new("{"),
            Err(AvroSerdeError::InvalidSchema { .. })
        ));
    }
}
