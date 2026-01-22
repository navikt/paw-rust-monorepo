FROM clux/muslrust:stable AS builder
WORKDIR /build
RUN cargo install cargo-auditable
COPY . .
ENV RUSTFLAGS='-C target-feature=+crt-static'

ARG APP
ENV BUILD_APP="cargo auditable build --target x86_64-unknown-linux-musl --release -p ${APP}"

RUN ${BUILD_APP}

FROM cgr.dev/chainguard/static:latest
WORKDIR /app
ARG APP
RUN ls /build/target/x86_64-unknown-linux-musl/release/
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/${APP} /app/${APP}
EXPOSE 8080
ENV RUN_APP=/app/${APP}
ENTRYPOINT ${RUN_APP}
