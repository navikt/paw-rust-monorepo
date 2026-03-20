use tracing::Level;
use tracing_subscriber::EnvFilter;

#[cfg(feature = "nais")]
const DEFAULT_LOG_LEVEL: Level = Level::INFO;
#[cfg(not(feature = "nais"))]
const DEFAULT_LOG_LEVEL: Level = Level::TRACE;

pub fn resolve_logging_filter() -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(DEFAULT_LOG_LEVEL.into())
        .from_env_lossy()
}

#[cfg(test)]
mod tests {
    use super::resolve_logging_filter;
    use tracing_subscriber::EnvFilter;

    #[cfg(feature = "nais")]
    const EXPECTED_DEFAULT_FILTER: &str = "info";
    #[cfg(not(feature = "nais"))]
    const EXPECTED_DEFAULT_FILTER: &str = "trace";

    #[test]
    fn test_resolve_logging_filter() {
        struct TestCase {
            navn: &'static str,
            rust_log: Option<&'static str>,
            forventet_filter: &'static str,
        }

        let test_cases = [
            TestCase {
                navn: "bruker default nar RUST_LOG mangler",
                rust_log: None,
                forventet_filter: EXPECTED_DEFAULT_FILTER,
            },
            TestCase {
                navn: "bruker default nar RUST_LOG er tom",
                rust_log: Some("   "),
                forventet_filter: EXPECTED_DEFAULT_FILTER,
            },
            TestCase {
                navn: "beholder gyldig direktiv nar en del av RUST_LOG er ugyldig",
                rust_log: Some("foo=IKKE_ET_NIVAA,my_crate=debug"),
                forventet_filter: "my_crate=debug",
            },
            TestCase {
                navn: "tolker target uten nivaa som trace",
                rust_log: Some("invalid_log_level,et::direktiv()=trace"),
                forventet_filter: "invalid_log_level=trace,et::direktiv()=trace",
            },
            TestCase {
                navn: "bruker RUST_LOG nar alle direktiver er gyldige",
                rust_log: Some("error,my_crate=debug"),
                forventet_filter: "error,my_crate=debug",
            },
        ];

        for test_case in test_cases {
            temp_env::with_var(EnvFilter::DEFAULT_ENV, test_case.rust_log, || {
                let resolved_filter = resolve_logging_filter();
                let expected_filter = EnvFilter::try_new(test_case.forventet_filter).unwrap();

                assert_eq!(
                    resolved_filter.to_string(),
                    expected_filter.to_string(),
                    "feilet testcase: {}",
                    test_case.navn
                );
            });
        }
    }
}

