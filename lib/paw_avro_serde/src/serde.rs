use apache_avro::types::Value;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;

use crate::error::AvroSerdeError;
pub use schema_registry_converter::async_impl::schema_registry::SrSettings;
pub use schema_registry_converter::schema_registry_common::SubjectNameStrategy;

#[derive(Clone, Debug)]
pub struct AvroDeserializer {
    decoder: Arc<schema_registry_converter::async_impl::avro::AvroDecoder<'static>>,
}

impl AvroDeserializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let decoder =
            schema_registry_converter::async_impl::avro::AvroDecoder::new(schema_reg_settings);
        Self {
            decoder: Arc::new(decoder),
        }
    }

    pub async fn deserialize<D: DeserializeOwned>(
        &self,
        payload: &[u8],
    ) -> Result<D, AvroSerdeError> {
        let value = decode_value(&self.decoder, payload).await?;
        let avro = apache_avro::from_value(&value)?;
        Ok(avro)
    }
}

#[derive(Clone, Debug)]
pub struct AvroStringDeserializer {
    decoder: Arc<schema_registry_converter::async_impl::avro::AvroDecoder<'static>>,
}

impl AvroStringDeserializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let decoder =
            schema_registry_converter::async_impl::avro::AvroDecoder::new(schema_reg_settings);
        Self {
            decoder: Arc::new(decoder),
        }
    }

    pub async fn deserialize(&self, payload: &[u8]) -> Result<String, AvroSerdeError> {
        match decode_value(&self.decoder, payload).await? {
            Value::String(value) => Ok(value),
            _ => Err(AvroSerdeError::AvroDeserializationFailed(
                "Expected Avro string value".to_string(),
            )),
        }
    }
}

async fn decode_value(
    decoder: &schema_registry_converter::async_impl::avro::AvroDecoder<'static>,
    payload: &[u8],
) -> Result<Value, AvroSerdeError> {
    let decoded = decoder.decode(Some(payload)).await.map_err(|e| {
        AvroSerdeError::AvroDeserializationFailed(format!(
            "Failed to deserialize Avro message with schema registry: {}",
            e
        ))
    })?;
    Ok(decoded.value)
}

#[derive(Clone, Debug)]
pub struct AvroSerializer {
    encoder: Arc<schema_registry_converter::async_impl::avro::AvroEncoder<'static>>,
}

impl AvroSerializer {
    pub fn new(schema_reg_settings: SrSettings) -> Self {
        let encoder =
            schema_registry_converter::async_impl::avro::AvroEncoder::new(schema_reg_settings);
        Self {
            encoder: Arc::new(encoder),
        }
    }

    pub async fn serialize(
        &self,
        avro: impl Serialize,
        strategy: &SubjectNameStrategy,
    ) -> Result<Vec<u8>, AvroSerdeError> {
        let payload = self
            .encoder
            .encode_struct(avro, strategy)
            .await
            .map_err(|e| {
                AvroSerdeError::AvroSerializationFailed(format!(
                    "Failed to serialize Avro message with schema registry: {}",
                    e
                ))
            })?;

        Ok(payload)
    }

    pub async fn serialize_value(
        &self,
        avro: impl Serialize,
        strategy: &SubjectNameStrategy,
    ) -> Result<Vec<u8>, AvroSerdeError> {
        let subject = strategy.get_subject().map_err(|e| {
            AvroSerdeError::AvroSerializationFailed(format!(
                "Failed to serialize Avro message with schema registry: {}",
                e
            ))
        })?;
        let schema = self
            .encoder
            .get_schema_and_id(&subject, strategy.clone())
            .await
            .map_err(|e| {
                AvroSerdeError::AvroSerializationFailed(format!(
                    "Failed to serialize Avro message with schema registry: {}",
                    e
                ))
            })?;
        let value = apache_avro::to_value(avro).and_then(|value| value.resolve(&schema.parsed))?;
        self.encoder
            .encode_value(value, strategy)
            .await
            .map_err(|e| {
                AvroSerdeError::AvroSerializationFailed(format!(
                    "Failed to serialize Avro message with schema registry: {}",
                    e
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use apache_avro::Schema;
    use mockito::Server;
    use schema_registry_converter::avro_common::get_supplied_schema;
    use schema_registry_converter::schema_registry_common::SubjectNameStrategy;
    use serde::{Deserialize, Serialize};

    const TOPIC: &str = "pdl.leesah-v1";
    const STRING_SCHEMA: &str = r#""string""#;
    const RECORD_SCHEMA: &str =
        r#"{"type":"record","name":"Sample","fields":[{"name":"id","type":"string"}]}"#;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Sample {
        id: String,
    }

    fn schema_id(bytes: &[u8]) -> u32 {
        assert_eq!(bytes[0], 0);
        u32::from_be_bytes(bytes[1..5].try_into().unwrap())
    }

    #[tokio::test]
    async fn round_trips_string_and_record_with_separate_schema_ids_and_subjects() {
        let mut server = Server::new_async().await;
        let _string_subject = server
            .mock("POST", "/subjects/pdl.leesah-v1-key/versions")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(r#"{"id":71}"#)
            .create_async()
            .await;
        let _record_subject = server
            .mock("POST", "/subjects/pdl.leesah-v1-value/versions")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(r#"{"id":72}"#)
            .create_async()
            .await;
        let _string_schema = server
            .mock("GET", "/schemas/ids/71?deleted=true")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(serde_json::json!({"schema": STRING_SCHEMA}).to_string())
            .create_async()
            .await;
        let _record_schema = server
            .mock("GET", "/schemas/ids/72?deleted=true")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(serde_json::json!({"schema": RECORD_SCHEMA}).to_string())
            .create_async()
            .await;
        let settings = SrSettings::new_builder(server.url())
            .no_proxy()
            .build()
            .unwrap();
        let serializer = AvroSerializer::new(settings.clone());
        let string_deserializer = AvroStringDeserializer::new(settings.clone());
        let deserializer = AvroDeserializer::new(settings);
        let key_strategy = SubjectNameStrategy::TopicNameStrategyWithSchema(
            TOPIC.to_string(),
            true,
            get_supplied_schema(&Schema::String),
        );
        let record_schema = Schema::parse_str(RECORD_SCHEMA).unwrap();
        let value_strategy = SubjectNameStrategy::TopicNameStrategyWithSchema(
            TOPIC.to_string(),
            false,
            get_supplied_schema(&record_schema),
        );

        let key = serializer
            .serialize("101701234500", &key_strategy)
            .await
            .unwrap();
        let value = serializer
            .serialize(
                Sample {
                    id: "sample-id".to_string(),
                },
                &value_strategy,
            )
            .await
            .unwrap();

        assert_eq!(schema_id(&key), 71);
        assert_eq!(schema_id(&value), 72);
        assert_eq!(
            string_deserializer.deserialize(&key).await.unwrap(),
            "101701234500"
        );
        assert_eq!(
            deserializer.deserialize::<Sample>(&value).await.unwrap(),
            Sample {
                id: "sample-id".to_string(),
            }
        );
        _string_subject.assert();
        _record_subject.assert();
    }

    #[tokio::test]
    async fn decodes_independently_encoded_avro_string_fixtures() {
        let mut server = Server::new_async().await;
        let _schema = server
            .mock("GET", "/schemas/ids/9?deleted=true")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(serde_json::json!({"schema": STRING_SCHEMA}).to_string())
            .create_async()
            .await;
        let settings = SrSettings::new_builder(server.url())
            .no_proxy()
            .build()
            .unwrap();
        let deserializer = AvroStringDeserializer::new(settings);
        for (datum, expected) in [
            (&[0][..], ""),
            (&[4, 195, 165][..], "å"),
            (&[12, 195, 169, 240, 159, 153, 130][..], "é🙂"),
        ] {
            let mut payload = vec![0, 0, 0, 0, 9];
            payload.extend_from_slice(datum);
            assert_eq!(deserializer.deserialize(&payload).await.unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn rejects_non_string_avro_types_as_keys() {
        let mut server = Server::new_async().await;
        let _schema = server
            .mock("GET", "/schemas/ids/10?deleted=true")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(serde_json::json!({"schema": r#""int""#}).to_string())
            .create_async()
            .await;
        let settings = SrSettings::new_builder(server.url())
            .no_proxy()
            .build()
            .unwrap();
        let deserializer = AvroStringDeserializer::new(settings);
        let error = deserializer
            .deserialize(&[0, 0, 0, 0, 10, 0])
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "Failed to deserialize Avro message: Expected Avro string value"
        );
    }

    #[tokio::test]
    async fn reports_framing_registry_and_datum_errors() {
        let mut server = Server::new_async().await;
        let _schema = server
            .mock("GET", "/schemas/ids/12?deleted=true")
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(serde_json::json!({"schema": STRING_SCHEMA}).to_string())
            .create_async()
            .await;
        let _registry_error = server
            .mock("GET", "/schemas/ids/13?deleted=true")
            .with_status(500)
            .with_body("registry failure")
            .create_async()
            .await;
        let _unknown_schema = server
            .mock("GET", "/schemas/ids/14?deleted=true")
            .with_status(404)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(r#"{"error_code":40403,"message":"Schema not found"}"#)
            .create_async()
            .await;
        let settings = SrSettings::new_builder(server.url())
            .no_proxy()
            .build()
            .unwrap();
        let deserializer = AvroStringDeserializer::new(settings);
        for payload in [&[0, 0][..], &[1, 0, 0, 0, 12][..]] {
            assert!(deserializer.deserialize(payload).await.is_err());
        }
        assert!(deserializer.deserialize(&[0, 0, 0, 0, 14]).await.is_err());
        assert!(
            deserializer
                .deserialize(&[0, 0, 0, 0, 13, 0])
                .await
                .is_err()
        );
        assert!(
            deserializer
                .deserialize(&[0, 0, 0, 0, 12, 128])
                .await
                .is_err()
        );
    }
}
