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
    use crate::api::arbeidsledighet::API_ARBEIDSLEDIGHET_PATH;
    use crate::api::kartlegging::API_KARTLEGGING_PATH;
    use crate::api::statistics::API_STATISTICS_PATH;
    use crate::model::dto::arbeidssoeker::{Arbeidssoeker, ArbeidssoekerKompakt};
    use crate::model::dto::bekreftelse::{Bekreftelse, Bekreftelsesloesning};
    use crate::model::dto::egenvurdering::Egenvurdering;
    use crate::model::dto::kontortilknytning::{KontorType, Kontortilknytning};
    use crate::model::dto::ledighetsperiode::{Ledighetsperiode, LedighetsperiodeKompakt};
    use crate::model::dto::opplysninger::{Jobbsituasjon, Opplysninger};
    use crate::model::dto::periode::Periode;
    use crate::model::dto::profilering::{Profilering, ProfilertTil};
    use crate::model::dto::request::{
        IdentitetsnummerQueryRequest, PagingRequest, QueryRequest, TilknyttetKontorQueryRequest,
    };
    use crate::model::dto::response::{
        ArbeidsledighetResponse, KartleggingResponse, PagingResponse, StatisticsResponse,
    };
    use crate::model::sort::SortOrder;
    use chrono::DateTime;
    use paw_error_handling::problem_details::ProblemDetails;
    use serde_json::json;
    use uuid::Uuid;

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

        let registered = [
            API_KARTLEGGING_PATH,
            API_ARBEIDSLEDIGHET_PATH,
            API_STATISTICS_PATH,
        ];

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
        let dto = QueryRequest::Identitetsnummer(IdentitetsnummerQueryRequest {
            identitetsnummer: "01017012345".to_string(),
            paging: Some(PagingRequest {
                page: 1,
                page_size: 10,
                sort_order: SortOrder::Ascending,
            }),
        });
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "QueryRequest", instance);
    }

    #[test]
    fn query_request_tilknyttet_kontor_konformerer() {
        let spec = spec();
        let dto = QueryRequest::TilknyttetKontor(TilknyttetKontorQueryRequest {
            kontor_id: "1337".to_string(),
            kontor_type: None,
            ledig_siden: None,
            paging: Some(PagingRequest {
                page: 1,
                page_size: 10,
                sort_order: SortOrder::Descending,
            }),
        });
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "QueryRequest", instance);
    }

    #[test]
    fn tom_kartlegging_response_konformerer() {
        let spec = spec();
        let dto = KartleggingResponse {
            arbeidssoekere: vec![],
            paging: PagingResponse {
                page: 1,
                page_size: 100,
                hit_size: 42,
                total_count: 1000,
                sort_order: SortOrder::Ascending,
            },
        };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "KartleggingResponse", instance);
    }

    #[test]
    fn arbeidssoeker_konformerer() {
        let spec = spec();
        let dto = Arbeidssoeker {
            id: 1337,
            aktor_id: "101701234500".to_string(),
            identitetsnummer: "01017012345".to_string(),
            fornavn: Some("Kari".to_string()),
            mellomnavn: None,
            etternavn: Some("Nordmann".to_string()),
            ledighetsperioder: vec![Ledighetsperiode {
                ledig_siden: Some(
                    DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                ),
                periode: Some(Periode {
                    id: Uuid::parse_str("069f40c9-c47c-4ee2-9105-bc87bdb58af2").unwrap(),
                    startet: DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                    avsluttet: None,
                }),
                opplysninger: Some(Opplysninger {
                    id: Uuid::parse_str("47c4b16b-5d34-4658-9705-ab90e6d0db9b").unwrap(),
                    jobbsituasjon: vec![Jobbsituasjon::AkkuratFullfortUtdanning],
                    tidspunkt: DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                }),
                profilering: Some(Profilering {
                    id: Uuid::parse_str("6d084994-d0b9-4466-9c1f-6126a3b3c2a8").unwrap(),
                    profilert_til: ProfilertTil::AntattGodeMuligheter,
                    tidspunkt: DateTime::parse_from_rfc3339("2021-01-01T12:02:00.000Z")
                        .unwrap()
                        .to_utc(),
                }),
                egenvurdering: Some(Egenvurdering {
                    id: Uuid::parse_str("7778ba2d-cbb5-4263-a49b-a4719821f0a5").unwrap(),
                    egenvurdert_til: ProfilertTil::OppgittHindringer,
                    tidspunkt: DateTime::parse_from_rfc3339("2021-01-02T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                }),
                bekreftelse: Some(Bekreftelse {
                    id: Uuid::parse_str("d8672838-145e-44d1-9334-dc6a706fa85c").unwrap(),
                    gjelder_fra: DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                    gjelder_til: DateTime::parse_from_rfc3339("2021-01-14T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                    har_jobbet: false,
                    vil_fortsette: true,
                    bekreftelsesloesning: Bekreftelsesloesning::Arbeidssoekerregisteret,
                }),
                bekreftelse_paa_vegne_av: vec![],
            }],
            kontortilknytninger: vec![Kontortilknytning {
                kontor_id: "1337".to_string(),
                kontor_navn: "NAV Bouvetøya".to_string(),
                kontor_type: KontorType::Arbeidsoppfolging,
            }],
        };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "Arbeidssoeker", instance);
    }

    fn finn_null_stier(value: &serde_json::Value, sti: &str, treff: &mut Vec<String>) {
        match value {
            serde_json::Value::Null => treff.push(sti.to_string()),
            serde_json::Value::Object(map) => {
                for (key, v) in map {
                    finn_null_stier(v, &format!("{sti}/{key}"), treff);
                }
            }
            serde_json::Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    finn_null_stier(v, &format!("{sti}/{i}"), treff);
                }
            }
            _ => {}
        }
    }

    fn assert_ingen_null(navn: &str, instance: &serde_json::Value) {
        let mut treff = Vec::new();
        finn_null_stier(instance, "", &mut treff);
        assert!(
            treff.is_empty(),
            "'{navn}' serialiserte null-verdier i stedet for å utelate feltene: {}",
            treff.join(", ")
        );
    }

    fn tom_ledighetsperiode() -> Ledighetsperiode {
        Ledighetsperiode {
            ledig_siden: None,
            periode: None,
            opplysninger: None,
            profilering: None,
            egenvurdering: None,
            bekreftelse: None,
            bekreftelse_paa_vegne_av: vec![],
        }
    }

    fn tom_ledighetsperiode_kompakt() -> LedighetsperiodeKompakt {
        LedighetsperiodeKompakt {
            periode_id: Uuid::parse_str("069f40c9-c47c-4ee2-9105-bc87bdb58af2").unwrap(),
            ledig_siden: None,
            periode_startet: DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                .unwrap()
                .to_utc(),
            periode_avsluttet: None,
            egenvurdert_til: None,
            bekreftelse_har_jobbet: None,
            bekreftelse_vil_fortsette: None,
            bekreftelse_paa_vegne_av: vec![],
        }
    }

    fn tom_paging_response() -> PagingResponse {
        PagingResponse {
            page: 1,
            page_size: 100,
            hit_size: 1,
            total_count: 1,
            sort_order: SortOrder::Descending,
        }
    }

    #[test]
    fn option_felt_utelates_fra_response_json() {
        let spec = spec();

        let kartlegging = KartleggingResponse {
            arbeidssoekere: vec![Arbeidssoeker {
                id: 1337,
                aktor_id: "101701234500".to_string(),
                identitetsnummer: "01017012345".to_string(),
                fornavn: None,
                mellomnavn: None,
                etternavn: None,
                ledighetsperioder: vec![tom_ledighetsperiode()],
                kontortilknytninger: vec![],
            }],
            paging: tom_paging_response(),
        };
        let instance = serde_json::to_value(&kartlegging).unwrap();
        assert_ingen_null("KartleggingResponse", &instance);
        assert_conforms(&spec, "KartleggingResponse", instance);

        let arbeidsledighet = ArbeidsledighetResponse {
            arbeidssoekere: vec![ArbeidssoekerKompakt {
                id: 1337,
                aktor_id: "101701234500".to_string(),
                identitetsnummer: "01017012345".to_string(),
                fornavn: None,
                mellomnavn: None,
                etternavn: None,
                ledighetsperioder: vec![tom_ledighetsperiode_kompakt()],
                kontortilknytninger: vec![],
            }],
            paging: tom_paging_response(),
        };
        let instance = serde_json::to_value(&arbeidsledighet).unwrap();
        assert_ingen_null("ArbeidsledighetResponse", &instance);
        assert_conforms(&spec, "ArbeidsledighetResponse", instance);

        let periode = Periode {
            id: Uuid::parse_str("069f40c9-c47c-4ee2-9105-bc87bdb58af2").unwrap(),
            startet: DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                .unwrap()
                .to_utc(),
            avsluttet: None,
        };
        let instance = serde_json::to_value(&periode).unwrap();
        assert_ingen_null("Periode", &instance);
        assert_conforms(&spec, "Periode", instance);

        let problem = ProblemDetails::internal_server_error("/api/v1/kartlegging", None);
        let instance = serde_json::to_value(&problem).unwrap();
        assert_ingen_null("ProblemDetails", &instance);
        assert_conforms(&spec, "ProblemDetails", instance);
    }

    #[test]
    fn option_felt_utelates_fra_request_json() {
        let spec = spec();
        let dto = QueryRequest::TilknyttetKontor(TilknyttetKontorQueryRequest {
            kontor_id: "1337".to_string(),
            kontor_type: None,
            ledig_siden: None,
            paging: None,
        });
        let instance = serde_json::to_value(&dto).unwrap();
        assert_ingen_null("TilknyttetKontorQueryRequest", &instance);
        assert_conforms(&spec, "QueryRequest", instance);
    }

    /// Request og response er bevisst asymmetriske.
    ///
    /// Serveren deserialiserer `Option`-felt med serde, som godtar både utelatt felt
    /// og eksplisitt `null`. Request-skjemaene er derfor nullable, ellers ville spec
    /// avvist forespørsler serveren faktisk godtar.
    ///
    /// Response-DTO-ene bruker `skip_serializing_none`, så `None` gir fravær av
    /// nøkkelen. Response-skjemaene er derfor ikke nullable.
    #[test]
    fn request_godtar_eksplisitt_null() {
        let spec = spec();
        let json = serde_json::json!({
            "type": "TILKNYTTET_KONTOR",
            "kontorId": "1337",
            "kontorType": null,
            "ledigSiden": null,
            "paging": null,
        });

        serde_json::from_value::<QueryRequest>(json.clone())
            .expect("serveren skal godta eksplisitt null i request");
        assert_conforms(&spec, "QueryRequest", json);
    }

    #[test]
    fn arbeidssoeker_kompakt_konformerer() {
        let spec = spec();
        let dto = ArbeidssoekerKompakt {
            id: 1337,
            aktor_id: "101701234500".to_string(),
            identitetsnummer: "01017012345".to_string(),
            fornavn: Some("Kari".to_string()),
            mellomnavn: None,
            etternavn: Some("Nordmann".to_string()),
            ledighetsperioder: vec![LedighetsperiodeKompakt {
                periode_id: Uuid::parse_str("069f40c9-c47c-4ee2-9105-bc87bdb58af2").unwrap(),
                ledig_siden: Some(
                    DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                        .unwrap()
                        .to_utc(),
                ),
                periode_startet: DateTime::parse_from_rfc3339("2021-01-01T12:00:00.000Z")
                    .unwrap()
                    .to_utc(),
                periode_avsluttet: None,
                egenvurdert_til: Some(ProfilertTil::OppgittHindringer),
                bekreftelse_har_jobbet: Some(false),
                bekreftelse_vil_fortsette: Some(true),
                bekreftelse_paa_vegne_av: vec![Bekreftelsesloesning::Dagpenger],
            }],
            kontortilknytninger: vec![Kontortilknytning {
                kontor_id: "1337".to_string(),
                kontor_navn: "NAV Bouvetøya".to_string(),
                kontor_type: KontorType::Arbeidsoppfolging,
            }],
        };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "ArbeidssoekerKompakt", instance);
    }

    #[test]
    fn statistics_response_konformerer() {
        let spec = spec();
        let dto = StatisticsResponse {
            total: 10000,
            is_null: 100,
            is_not_null: 9900,
            over_0030_days: 8000,
            over_0060_days: 6000,
            over_0090_days: 4000,
            over_0180_days: 2000,
            over_0365_days: 1000,
            over_0730_days: 500,
            over_1095_days: 250,
        };
        let instance = serde_json::to_value(&dto).unwrap();
        assert_conforms(&spec, "StatisticsResponse", instance);
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
        for path in [API_KARTLEGGING_PATH, API_ARBEIDSLEDIGHET_PATH] {
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
