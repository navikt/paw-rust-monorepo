FROM clux/muslrust:1.98.0-stable@sha256:5fb7882c8a6be209729342039f9ca014143b8200d887b484c32a0d79e54d0f59 AS chef
WORKDIR /build
RUN cargo --version && cargo install --locked cargo-chef cargo-auditable
# Må stå etter cargo install, ellers bygges verktøyene med crt-static, og før
# cook, ellers ser cook og det endelige bygget ulike flagg og cachen bommer.
ENV RUSTFLAGS='-C target-feature=+crt-static'

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
# rust-toolchain.toml må være på plass før cook, ellers kjører cook på imagets
# toolchain og det endelige bygget på den pinnede.
COPY rust-toolchain.toml rust-toolchain.toml
# Bygger bare avhengighetene. Laget er nøkkelt på recipe.json, som utledes av
# Cargo.lock og manifestene, så det overlever enhver endring i kildekode.
# Kommandoen nevner verken app eller features, så alle appene deler ett lag.
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY . .
ARG APP
ARG FEATURES
ARG GIT_COMMIT_HASH=dev-build
ENV GIT_COMMIT_HASH=${GIT_COMMIT_HASH}
RUN cargo auditable build --target x86_64-unknown-linux-musl --release -p ${APP} ${FEATURES:+--features ${FEATURES}}

FROM cgr.dev/chainguard/static:latest
WORKDIR /app
ARG APP
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/${APP} /app/app
EXPOSE 8080
ENTRYPOINT ["/app/app"]
