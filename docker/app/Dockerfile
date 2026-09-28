# App-image bygget på det forhåndskompilerte builder-imaget.
#
# Brukes foreløpig kun av internkontroll-piloten. De øvrige appene bygger
# fortsatt med Dockerfile i repo-roten, som koker avhengighetene selv.
# Rollback er å peke workflowen tilbake på den.
#
# BUILDER_IMAGE settes av CI til en digest-pinnet referanse. Defaulten gjelder
# lokale bygg, der `just docker-build` bygger builder-imaget først.
ARG BUILDER_IMAGE=paw-rust-builder:local

FROM ${BUILDER_IMAGE} AS builder
WORKDIR /build
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
