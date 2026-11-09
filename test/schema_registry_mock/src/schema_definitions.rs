use schema_registry_converter::schema_registry_common::{SchemaType, SuppliedSchema};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fmt;

#[derive(Serialize, Deserialize)]
pub struct AvroSchema {
    id: i32,
    version: i32,
    subject: &'static str,
    schema: &'static str,
}

impl AvroSchema {
    pub fn subject(&self) -> String {
        self.subject.to_owned()
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
        serde_json::to_string(&self).unwrap()
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
            .field("subject", &self.subject)
            .finish()
    }
}

pub fn avro_schemas() -> Vec<AvroSchema> {
    vec![
        AvroSchema {
            id: 1,
            version: 1,
            subject: "paw.arbeidssokerperioder-v1-value",
            schema: include_str!("../avro/json/arbeidssoekerregisteret/periode.json"),
        },
        AvroSchema {
            id: 2,
            version: 1,
            subject: "paw.opplysninger-om-arbeidssoeker-v1-value",
            schema: include_str!("../avro/json/arbeidssoekerregisteret/opplysninger.json"),
        },
        AvroSchema {
            id: 3,
            version: 1,
            subject: "paw.arbeidssoker-profilering-v1-value",
            schema: include_str!("../avro/json/arbeidssoekerregisteret/profilering.json"),
        },
        AvroSchema {
            id: 4,
            version: 1,
            subject: "paw.arbeidssoeker-egenvurdering-v1-value",
            schema: include_str!("../avro/json/arbeidssoekerregisteret/egenvurdering.json"),
        },
        AvroSchema {
            id: 5,
            version: 1,
            subject: "paw.arbeidssoker-bekreftelse-v1-value",
            schema: include_str!("../avro/json/arbeidssoekerregisteret/bekreftelse.json"),
        },
        AvroSchema {
            id: 6,
            version: 1,
            subject: "paw.arbeidssoker-bekreftelse-paavegneav-v1-value",
            schema: include_str!(
                "../avro/json/arbeidssoekerregisteret/bekreftelse-paavegneav.json"
            ),
        },
        AvroSchema {
            id: 7,
            version: 1,
            subject: "pdl.leesah-v1-key",
            schema: include_str!("../avro/json/common/string.json"),
        },
        AvroSchema {
            id: 8,
            version: 1,
            subject: "pdl.leesah-v1-value",
            schema: include_str!("../avro/json/pdl/personhendelse.json"),
        },
    ]
}
