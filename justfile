# List directory tree (folders only, git-tracked)
list-folders:
    tree --gitignore -d

# List directory tree (with files, git-tracked)
list:
    tree --gitignore

# List directory tree for a specific app  (e.g. just list-app utgang)
list-app app:
    tree --gitignore app/{{ app }}

# Show available apps (discovered from app/ directory)
apps:
    @ls -1 app/

# Build workspace or a specific app  (e.g. just build utgang)
build app="":
    cargo build {{ if app == "" { "--workspace" } else { "-p " + app } }}

# Quick check without full compilation  (e.g. just check utgang)
check app="":
    cargo check {{ if app == "" { "--workspace" } else { "-p " + app } }}

# Run tests for workspace or a specific app  (e.g. just test utgang)
test app="":
    cargo test {{ if app == "" { "--workspace" } else { "-p " + app } }}

# Run clippy lints  (e.g. just clippy utgang)
clippy app="":
    cargo clippy {{ if app == "" { "--workspace" } else { "-p " + app } }} -- -D warnings

# Format all code
fmt:
    cargo fmt --all

# Check formatting without modifying files
fmt-check:
    cargo fmt --all -- --check

# Full CI gate: fmt-check + clippy + test
ci: fmt-check clippy test

# Start local Postgres
postgres-up:
    docker compose -f docker/postgres/docker-compose.yaml up -d

# Stop local Postgres
postgres-down:
    docker compose -f docker/postgres/docker-compose.yaml down

# Start local Kafka + Schema Registry + UI
kafka-up:
    docker compose -f docker/kafka/docker-compose.yaml up -d

# Stop local Kafka
kafka-down:
    docker compose -f docker/kafka/docker-compose.yaml down

# Start all local infrastructure
infra-up: postgres-up kafka-up

# Stop all local infrastructure
infra-down: postgres-down kafka-down

# Build Docker image for an app  (e.g. just docker-build utgang)
docker-build app features="":
    docker build \
        --build-arg APP={{ app }} \
        {{ if features == "" { "" } else { "--build-arg FEATURES=" + features } }} \
        -t {{ app }}:local \
        .
