FROM clux/muslrust:stable AS builder
WORKDIR /build
RUN cargo install cargo-auditable
COPY . .
ENV RUSTFLAGS='-C target-feature=+crt-static'

ARG APP
ENV BUILD_APP="cargo auditable build --target x86_64-unknown-linux-musl --release -p ${APP}"
RUN echo build_cmd=${BUILD_APP}
RUN ${BUILD_APP}
RUN ls -l /build/target/x86_64-unknown-linux-musl/release/


FROM cgr.dev/chainguard/static:latest
WORKDIR /app
ARG APP
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/${APP} /app/app
EXPOSE 8080
ENTRYPOINT ["/app/app"]
