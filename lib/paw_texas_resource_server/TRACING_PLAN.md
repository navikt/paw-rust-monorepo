# Tracing Plan: OTel span-hendelser i texas_middleware

## Mål

Emitte to OpenTelemetry span-hendelser i `texas_middleware`:
- én ved funksjonsstart med `duration = 0`
- én ved fullføring med faktisk antall millisekunder elapsed

Hendelsene skal legges på den eksisterende OTel-spanen (ikke som logg-hendelser).

## Avhengigheter som må legges til

`lib/paw_texas_resource_server/Cargo.toml`:

```toml
opentelemetry = { workspace = true }
tracing-opentelemetry = { workspace = true }
```

## Implementasjon

`lib/paw_texas_resource_server/src/middleware.rs`:

```rust
use opentelemetry::KeyValue;
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[tracing::instrument]
pub async fn texas_middleware(
    State(state): State<Arc<AuthState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, ProblemDetails> {
    let start = std::time::Instant::now();
    let span = tracing::Span::current();
    span.add_event("texas_middleware started", vec![KeyValue::new("duration", 0i64)]);

    let result = async {
        // ... all existing logic unchanged ...
    }.await;

    span.add_event(
        "texas_middleware completed",
        vec![KeyValue::new("duration", start.elapsed().as_millis() as i64)],
    );
    result
}
```

### Viktige detaljer

- `tracing::Span::current()` henter den aktive spanen som `#[tracing::instrument]` oppretter.
- Eksisterende logikk (linje 21–133) flyttes inn i `async { ... }.await`-blokken slik at `?`-operatorer propagerer inni blokken og gir ett enkelt exit-punkt etter blokken.
- `KeyValue::new("duration", value as i64)` — OTel `Value` implementerer `From<i64>`.
- `as_millis()` returnerer `u128`; cast til `i64` er trygt for realistiske middleware-varigheter.

## Verifisering

```sh
just clippy paw_texas_resource_server
just test paw_texas_resource_server
```
