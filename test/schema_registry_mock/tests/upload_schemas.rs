use schema_registry_converter::async_impl::schema_registry::{post_schema, SrSettings};
use schema_registry_mock::schema_definitions::avro_schemas;

#[ignore]
#[tokio::test]
async fn upload_schemas() -> anyhow::Result<()> {
    let base_url = "http://localhost:8082";
    let sr_settings = SrSettings::new_builder(base_url.to_string())
        .no_proxy()
        .build()?;
    let http_client = reqwest::Client::builder().no_proxy().build()?;

    for schema in avro_schemas() {
        let config_url = format!("{}{}", base_url, schema.subject_config_path());
        let response = http_client
            .put(&config_url)
            .json(&schema.subject_config_request_body())
            .send()
            .await?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            eprintln!("Config PUT feilet med {status}: {body}");
        }

        let registered = post_schema(&sr_settings, schema.subject(), schema.to_supplied_schema()).await?;
        println!("Lastet opp {}: id={}", schema.subject(), registered.id);
    }

    Ok(())
}
