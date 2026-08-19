use schema_registry_converter::schema_registry_common::{SchemaType, SuppliedSchema};
use serde_json::{Value, json};
use std::fmt;

pub struct AvroSchema {
    pub id: i32,
    pub version: i32,
    pub topic: &'static str,
    pub schema: &'static str,
}

impl AvroSchema {
    pub fn subject(&self) -> String {
        format!("{}-value", self.topic)
    }

    pub fn subject_config_path(&self) -> String {
        format!("/config/{}", self.subject())
    }

    pub fn subject_version_path(&self) -> String {
        format!("/subjects/{}/versions/latest", self.subject())
    }

    pub fn schema_id_path(&self) -> String {
        format!("/schemas/ids/{}?deleted=true", self.id)
    }

    pub fn subject_config_request_body(&self) -> Value {
        json!({ "compatibilityLevel": "FULL_TRANSITIVE" })
    }

    pub fn subject_version_response_body(&self) -> String {
        json!({
            "subject": format!("{}-value", self.topic),
            "version": self.version,
            "id": self.id,
            "schema": self.schema
        })
        .to_string()
    }

    pub fn schema_id_response_body(&self) -> String {
        json!({ "schema": self.schema }).to_string()
    }

    pub fn to_supplied_schema(&self) -> SuppliedSchema {
        SuppliedSchema {
            name: Some(self.subject()),
            schema_type: SchemaType::Avro,
            schema: self.schema.to_string(),
            references: vec![],
            properties: None,
            tags: None,
        }
    }
}

impl fmt::Debug for AvroSchema {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AvroSchema")
            .field("id", &self.id)
            .field("version", &self.version)
            .field("topic", &self.topic)
            .finish()
    }
}

pub fn avro_schemas() -> Vec<AvroSchema> {
    vec![
        AvroSchema {
            id: 1,
            version: 1,
            topic: "paw.arbeidssokerperioder-v1",
            schema: include_str!("../schemas/periode.json"),
        },
        AvroSchema {
            id: 2,
            version: 1,
            topic: "paw.opplysninger-om-arbeidssoeker-v1",
            schema: include_str!("../schemas/opplysninger.json"),
        },
        AvroSchema {
            id: 3,
            version: 1,
            topic: "paw.arbeidssoker-profilering-v1",
            schema: include_str!("../schemas/profilering.json"),
        },
        AvroSchema {
            id: 4,
            version: 1,
            topic: "paw.arbeidssoeker-egenvurdering-v1",
            schema: include_str!("../schemas/egenvurdering.json"),
        },
        AvroSchema {
            id: 5,
            version: 1,
            topic: "paw.arbeidssoker-bekreftelse-v1",
            schema: include_str!("../schemas/bekreftelse.json"),
        },
        AvroSchema {
            id: 6,
            version: 1,
            topic: "paw.arbeidssoker-bekreftelse-paavegneav-v1",
            schema: include_str!("../schemas/bekreftelse-paavegneav.json"),
        },
    ]
}
