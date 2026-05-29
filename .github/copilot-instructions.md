# Workflow

One atomic change per commit. Every commit must be deployable (build + tests pass).

1. View a file before editing — it may have changed
2. Make a single logical change
3. `git pull --rebase` before building
4. Build and test the affected module(s)
5. Commit with a descriptive message, then stop

Commit messages: imperative, Norwegian, keep technical terms in English. Example: `Fjern KafkaMessage, bruk OwnedMessage direkte`. Never push — only commit locally.

# Build, Test, Lint

```sh
just build utgang        # cargo build -p utgang
just test utgang         # cargo nextest run -p utgang
just clippy utgang       # cargo clippy -p utgang -- -D warnings
just fmt                 # cargo fmt --all
just ci                  # fmt-check + clippy + test (full workspace)
```

Omit the app name to run against the entire workspace. Single test: `cargo nextest run -p MODULE -- test_name`.

Local infrastructure: `just infra-up` (Postgres + Kafka), `just infra-down`.

# Architecture

Rust 2024 edition monorepo deployed to Nav's Nais (Kubernetes/GCP). Static musl binaries in distroless containers.

```
app/     — deployable applications (Kafka consumers, HTTP servers)
lib/     — shared libraries (no deps on app crates)
domain/  — domain structs, minimal logic, no deps on lib or app
```

**Stack:** Tokio, axum, reqwest, rdkafka, sqlx (Postgres), tracing + OpenTelemetry.

## App structure convention

Each app follows this layout:

```
app/<name>/
├── Cargo.toml          # `features = ["nais"]` for env-based config
├── config/
│   ├── local/          # TOML configs for local dev
│   └── nais/           # TOML configs with ${ENV_VAR} interpolation
├── migrations/         # sqlx migrations
├── nais/               # Kubernetes manifests (nais-dev.yaml, nais-prod.yaml)
├── src/
│   ├── main.rs         # Entrypoint: setup tracing, DB, Kafka, axum health server
│   └── lib.rs          # All business logic exposed as a library
└── tests/              # Integration tests
```

## Configuration pattern

Config is compile-time selected via the `nais` feature flag using `read_config_file!("name.toml")`:
- Without feature: reads from `config/local/`
- With `nais` feature: reads from `config/nais/` (env vars like `${NAIS_DATABASE_..._HOST}` are resolved at runtime by serde-env-field)

## Key libraries

| Crate | Purpose |
|-------|---------|
| `axum_health` | Health endpoints (`/isalive`, `/isready`) + Prometheus metrics |
| `paw_rdkafka_hwm` | High-water-mark Kafka consumer with sqlx-backed offset tracking |
| `texas_client` | Nais Texas token operations (M2M, exchange, validation) |
| `paw_sqlx` | Postgres pool init and helpers |
| `paw_app_config` | `read_config_file!` macro for TOML config loading |

`azure_m2m_client` is deprecated — use `texas_client` for all auth.

## Workspace dependencies

All dependencies are declared in the root `Cargo.toml` under `[workspace.dependencies]`. App crates reference them with `{ workspace = true }`. Add new deps there, not in individual Cargo.toml files.

# Conventions

- **Error handling:** `thiserror` for domain errors, `anyhow` in main/entrypoints only. No `.unwrap()`/`.expect()` outside tests.
- **Dispatch:** Prefer static (`impl Trait`, generics) over dynamic (`dyn Trait`).
- **Shared state:** Wrap inner state in `Arc` so the outer handle is cheaply cloneable. Use atomics for simple flags/counters.
- **Observability:** `tracing` crate with structured fields. JSON to stdout + OpenTelemetry gRPC. Use `#[instrument]` on key async functions.
- **No PII in logs** — log correlation IDs (periodeId, hendelseId), never fnr/name.
- **No in-code comments** unless something is genuinely non-obvious.
- **Testing:** Test new behavior and business logic. Don't test library internals or add tests for pure refactoring. Use `mockito` for HTTP mocks, `testcontainers` for Postgres integration tests.
