#!/bin/bash
export RUST_LOG=info
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
export OTEL_SERVICE_NAME=test-app
export NAIS_NAMESPACE=local

cargo build -p test-app --quiet

timeout 3s cargo run -p test-app