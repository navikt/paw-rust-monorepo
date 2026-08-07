FROM clux/muslrust:1.97.1-stable@sha256:4edc98b7a3a627389f6d9dbf91c0dbe9a715378239797a923c86e365bb24a435 AS builder
WORKDIR /build
RUN cargo --version && cargo install cargo-auditable
COPY . .
ENV RUSTFLAGS='-C target-feature=+crt-static'

ARG APP
ARG GIT_COMMIT_HASH=dev-build
ARG FEATURES
ENV BUILD_APP="cargo auditable build --target x86_64-unknown-linux-musl --release -p ${APP} ${FEATURES:+--features ${FEATURES}}"
ENV GIT_COMMIT_HASH=${GIT_COMMIT_HASH}
RUN echo "build_cmd=${BUILD_APP}"
RUN ${BUILD_APP}
RUN ls -l /build/target/x86_64-unknown-linux-musl/release/

FROM cgr.dev/chainguard/static:latest
WORKDIR /app
ARG APP
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/${APP} /app/app
EXPOSE 8080
ENTRYPOINT ["/app/app"]
