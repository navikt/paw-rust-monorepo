use crate::schema_definitions::avro_schemas;

// TODO: Lage klient for å laste opp Avro-schemas
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let base_url = "http://localhost:8082";
    let http_client = reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("Failed to build reqwest client");

    let schemas = avro_schemas();
    for schema in schemas {
        let subject_config_url = format!("{}{}", base_url, schema.subject_config_path());
        let subject_config_request = schema.subject_config_request_body();
        let response = http_client
            .put(subject_config_url)
            .json(&subject_config_request)
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::OK => {
                println!("Schemas uploaded successfully");
            }
            _ => {
                let status = response.status();
                let text = response.text().await.unwrap_or_default();
                eprintln!("Schemas upload failed with status {}: {}", status, text);
            }
        }
    }

    Ok(())
}
