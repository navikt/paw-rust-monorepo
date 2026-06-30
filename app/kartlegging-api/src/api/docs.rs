use axum::http::header::{ACCEPT, CONTENT_TYPE};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use std::sync::OnceLock;

static SPEC_YAML: &str = include_str!("../../openapi/spec.yaml");
static SPEC_JSON: OnceLock<String> = OnceLock::new();

fn spec_json() -> &'static str {
    SPEC_JSON.get_or_init(|| {
        let value: serde_json::Value =
            serde_yaml::from_str(SPEC_YAML).expect("openapi/spec.yaml er gyldig YAML");
        serde_json::to_string(&value).expect("spec kan serialiseres til JSON")
    })
}

pub(crate) async fn api_docs(headers: HeaderMap) -> impl IntoResponse {
    let accept = headers
        .get(ACCEPT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    tracing::info!("API docs type: {}", accept);
    if accept.contains("application/yaml") {
        (
            [(CONTENT_TYPE, "application/yaml; charset=utf-8")],
            SPEC_YAML,
        )
            .into_response()
    } else {
        let value: serde_json::Value =
            serde_json::from_str(spec_json()).expect("spec_json() er gyldig JSON");
        Json(value).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::{spec_json, SPEC_YAML};
    use serde_json::json;

    fn spec() -> serde_json::Value {
        serde_json::from_str(spec_json()).expect("spec_json() er gyldig JSON")
    }

    fn assert_conforms(spec: &serde_json::Value, schema_name: &str, instance: serde_json::Value) {
        let mut schema = spec.clone();
        schema["$ref"] = json!(format!("#/components/schemas/{schema_name}"));
        let validator = jsonschema::draft202012::options()
            .build(&schema)
            .unwrap_or_else(|e| panic!("Kunne ikke bygge validator for '{schema_name}': {e}"));
        let errors: Vec<_> = validator
            .iter_errors(&instance)
            .map(|e| format!("  - {e} (ved {})", e.instance_path()))
            .collect();
        assert!(
            errors.is_empty(),
            "Instans konformerer ikke med spec-skjema '{schema_name}':\n{}",
            errors.join("\n")
        );
    }

    #[test]
    fn spec_yaml_er_gyldig_yaml() {
        let value: serde_json::Value =
            serde_yaml::from_str(SPEC_YAML).expect("spec.yaml skal være gyldig YAML");
        assert!(value.is_object());
    }

    #[test]
    fn spec_paths_match_registered_routes() {
        let spec = spec();
        let spec_paths: Vec<String> = spec["paths"]
            .as_object()
            .expect("spec.paths er et objekt")
            .keys()
            .cloned()
            .collect();

        let registered = ["/api/v1/oversikt", "/api/v1/kartlegging"];

        for path in &spec_paths {
            assert!(
                registered.contains(&path.as_str()),
                "Spec-sti '{path}' er ikke registrert i axum-routeren"
            );
        }
        for route in &registered {
            assert!(
                spec_paths.iter().any(|p| p == route),
                "Axum-rute '{route}' mangler i spec"
            );
        }
    }

    #[test]
    fn tom_oversikt_response_konformerer() {
        let spec = spec();
        assert_conforms(
            &spec,
            "OversiktResponse",
            json!({
                "arbeidssoekere": [],
                "paging": {
                    "page": 1,
                    "pageSize": 10,
                    "hitSize": 0,
                    "totalCount": 0,
                    "sortOrder": "ASC"
                }
            }),
        );
    }

    #[test]
    fn query_request_identitetsnummer_konformerer() {
        let spec = spec();
        assert_conforms(
            &spec,
            "QueryRequest",
            json!({
                "type": "IDENTITETSNUMMER",
                "identitetsnummer": "01017012345"
            }),
        );
    }

    #[test]
    fn query_request_tilknyttet_kontor_konformerer() {
        let spec = spec();
        assert_conforms(
            &spec,
            "QueryRequest",
            json!({
                "type": "TILKNYTTET_KONTOR",
                "kontorId": "0301"
            }),
        );
    }

    #[test]
    fn arbeidssoeker_konformerer() {
        let spec = spec();
        assert_conforms(
            &spec,
            "Arbeidssoeker",
            json!({
                "arbeidssoekerId": 1_i64,
                "identitetsnummer": "01017012345",
                "fornavn": "Ola",
                "etternavn": "Nordmann",
                "periode": {
                    "id": "00000000-0000-0000-0000-000000000001",
                    "startet": "2026-01-01T00:00:00Z"
                },
                "bekreftelsePaaVegneAv": [],
                "tilknyttetKontor": []
            }),
        );
    }
}
