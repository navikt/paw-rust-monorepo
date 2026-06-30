use crate::schema_definitions::avro_schemas;
use mockito::ServerGuard;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use std::error::Error;

pub async fn create_schema_registry_mock(
    mockito_server: &mut ServerGuard,
) -> Result<SrSettings, Box<dyn Error>> {
    for schema in avro_schemas() {
        println!("Creating schema registry mock for schema {:?}", schema);
        let _subject_endpoint_mock = mockito_server
            .mock("GET", schema.subject_path().as_str())
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(schema.subject_response_body())
            .create_async()
            .await;

        let _schemas_endpoint_mock = mockito_server
            .mock("GET", schema.schema_path().as_str())
            .with_status(200)
            .with_header("content-type", "application/vnd.schemaregistry.v1+json")
            .with_body(schema.schema_response_body())
            .create_async()
            .await;
    }

    let schema_registry_settings = SrSettings::new_builder(mockito_server.url())
        .no_proxy()
        .build()?;
    Ok(schema_registry_settings)
}
