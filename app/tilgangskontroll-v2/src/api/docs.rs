use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use std::sync::OnceLock;
use utoipa_swagger_ui::{Config, SwaggerUi};

static SPEC_YAML: &str = include_str!("../../openapi/spec.yaml");
static SPEC_JSON: OnceLock<String> = OnceLock::new();

pub(crate) fn routes() -> Router {
    let docs_json_routes = Router::new().route("/api/docs.json", get(api_docs_json));
    let docs_yaml_routes = Router::new().route("/api/docs.yaml", get(api_docs_yaml));
    let swagger_routes =
        Router::from(SwaggerUi::new("/api/docs").config(Config::from("/api/docs.yaml")));
    docs_json_routes
        .merge(docs_yaml_routes)
        .merge(swagger_routes)
}

async fn api_docs_json(_: HeaderMap) -> impl IntoResponse {
    let value: serde_json::Value =
        serde_json::from_str(spec_json()).expect("spec_json() er gyldig JSON");
    Json(value).into_response()
}

async fn api_docs_yaml(_: HeaderMap) -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "application/yaml; charset=utf-8")],
        SPEC_YAML,
    )
        .into_response()
}

fn spec_json() -> &'static str {
    SPEC_JSON.get_or_init(|| {
        let value: serde_json::Value =
            serde_yaml::from_str(SPEC_YAML).expect("openapi/spec.yaml er gyldig YAML");
        serde_json::to_string(&value).expect("spec kan serialiseres til JSON")
    })
}

#[cfg(test)]
mod tests {
    use super::{SPEC_YAML, spec_json};
    use crate::api::v1::nav_enhet_tilgang::API_KONTOR_TILGANG_PATH;
    use crate::api::v1::person_tilgang::API_PERSON_TILGANG_PATH;
    use crate::model::dto::request::{
        NavAnsattNavEnhetTilgangRequest, NavAnsattPersonTilgangRequest,
    };
    use crate::model::dto::response::TilgangResponse;
    use crate::model::tilgang::TilgangType;
    use paw_error_handling::problem_details::ProblemDetails;
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

        let registered = [API_PERSON_TILGANG_PATH, API_KONTOR_TILGANG_PATH];

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
    fn query_request_identitetsnummer_konformerer() {
        let spec = spec();
        let dto = NavAnsattPersonTilgangRequest {
            tilgang_type: TilgangType::Lese,
            nav_ident: "Z123456".to_string(),
            identitetsnummer: "01017012345".to_string(),
        };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "NavAnsattPersonTilgangRequest", instance);
    }

    #[test]
    fn query_request_tilknyttet_kontor_konformerer() {
        let spec = spec();
        let dto = NavAnsattNavEnhetTilgangRequest {
            nav_ident: "Z123456".to_string(),
            nav_enhet_id: "1234".to_string(),
        };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "NavAnsattNavEnhetTilgangRequest", instance);
    }

    #[test]
    fn tilgang_response_konformerer() {
        let spec = spec();
        let dto = TilgangResponse { har_tilgang: true };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "TilgangResponse", instance);
    }

    #[test]
    fn problem_details_konformerer() {
        let spec = spec();
        let dto = ProblemDetails::validation_error("/api/v1/kartlegging", "Ugyldig request body");
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "ProblemDetails", instance);
    }

    #[test]
    fn spec_eksempler_konformerer_med_egne_skjemaer() {
        let spec = spec();
        let skjemaer = spec["components"]["schemas"]
            .as_object()
            .expect("components.schemas er et objekt");
        for (navn, skjema) in skjemaer {
            if let Some(eksempel) = skjema.get("example") {
                assert_conforms(&spec, navn, eksempel.clone());
            }
        }
    }

    #[test]
    fn beskyttede_operasjoner_krever_bearer_auth() {
        let spec = spec();
        for path in [API_PERSON_TILGANG_PATH, API_KONTOR_TILGANG_PATH] {
            let security = &spec["paths"][path]["post"]["security"];
            assert!(
                security
                    .as_array()
                    .is_some_and(|krav| krav.iter().any(|k| k.get("BearerAuth").is_some())),
                "Operasjonen på '{path}' mangler 'security: BearerAuth', men ligger bak oauth2_middleware"
            );
        }
    }
}
