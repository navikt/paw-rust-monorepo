use crate::schema_definitions::avro_schemas;
use mockito::{Mock, ServerGuard};
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::error::Error;

pub struct SchemaRegistryMockGuard {
    pub schema_registry_settings: SrSettings,
    pub mocks: Vec<Mock>,
}

pub async fn create_schema_registry_mock(
    mockito_server: &mut ServerGuard,
) -> Result<SchemaRegistryMockGuard, Box<dyn Error>> {
    let mut mocks = vec![];
    for schema in avro_schemas() {
        println!("Creating schema registry mock for schema {:?}", schema);
        mocks.push(
            mockito_server
                .mock("GET", schema.subject_path().as_str())
                .with_status(200)
                .with_header("content-type", "application/vnd.schemaregistry.v1+json")
                .with_body(schema.subject_response_body())
                .create_async()
                .await,
        );

        mocks.push(
            mockito_server
                .mock("GET", schema.schema_path().as_str())
                .with_status(200)
                .with_header("content-type", "application/vnd.schemaregistry.v1+json")
                .with_body(schema.schema_response_body())
                .create_async()
                .await,
        );
    }

    let schema_registry_settings = SrSettings::new_builder(mockito_server.url())
        .no_proxy()
        .build()?;
    Ok(SchemaRegistryMockGuard {
        schema_registry_settings,
        mocks,
    })
}
