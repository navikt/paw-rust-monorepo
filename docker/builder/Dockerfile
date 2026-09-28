# Byggeverktøy-image for paw-rust-monorepo.
#
# Dette imaget deployes aldri. Det brukes kun som FROM-base når appene bygges,
# og inneholder ferdig kompilerte avhengigheter pluss en varm registry-cache.
#
# Taggen er en innholdshash av Cargo.lock, alle Cargo.toml, rust-toolchain.toml,
# denne fila og hvilken app som kokes. Endres noe av dette, peker app-bygget på
# en annen tag, og workflowen bygger imaget på nytt. Et utdatert image kan
# derfor ikke tas i bruk ved en feil.
FROM clux/muslrust:1.98.0-stable@sha256:5fb7882c8a6be209729342039f9ca014143b8200d887b484c32a0d79e54d0f59 AS chef
WORKDIR /build
# Versjonene pinnes fordi taggen til dette imaget er en hash av fila, ikke av
# resultatet. Uten pinning kunne et gjenoppbygd image under samme tag fått
# andre verktøyversjoner, og cargo-auditable skriver avhengighetsmanifestet
# inn i binæren som havner i prod.
RUN cargo --version && cargo install --locked cargo-chef@0.1.78 cargo-auditable@0.7.6
# Må stå etter cargo install, ellers bygges verktøyene med crt-static, og før
# cook, ellers ser cook og det endelige bygget ulike flagg og cachen bommer.
ENV RUSTFLAGS='-C target-feature=+crt-static'

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# rust-toolchain.toml må være på plass før cook, ellers kjører cook på imagets
# toolchain og det endelige bygget på den pinnede.
COPY rust-toolchain.toml rust-toolchain.toml
COPY --from=planner /build/recipe.json recipe.json

ARG BUILDER_APP
ARG BUILDER_FEATURES
# Kokes med samme -p og --features som det endelige bygget bruker. Ulik
# feature-oppløsning gir ulik metadata-hash i cargo, og da bommer cachen selv
# om docker-lagene treffer. Cook henter også ned alle crates, så
# registry-cachen er varm i det ferdige imaget.
RUN cargo chef cook --release \
    --target x86_64-unknown-linux-musl \
    --recipe-path recipe.json \
    -p ${BUILDER_APP} \
    ${BUILDER_FEATURES:+--features ${BUILDER_FEATURES}}
