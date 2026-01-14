FROM clux/muslrust:stable AS builder
WORKDIR /build
RUN cargo install cargo-auditable
COPY . .
ENV RUSTFLAGS='-C target-feature=+crt-static'

RUN cargo auditable build --target x86_64-unknown-linux-musl --release -p test-app

FROM cgr.dev/chainguard/static:latest
WORKDIR /app
ARG APP
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/test-app /app/test-app
EXPOSE 8080
ENTRYPOINT ["/app/app"]
