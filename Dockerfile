# syntax=docker/dockerfile:1
FROM oven/bun:1.4.2 AS web
WORKDIR /src/web
COPY web/package.json web/bun.lock ./
RUN bun install --frozen-lockfile
COPY web/ ./
RUN bun run build

FROM rust:1-bookworm AS builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY deploy/ ./deploy/
COPY plugins/ ./plugins/
COPY --from=web /src/web/dist ./web/dist
ARG CARGO_BUILD_JOBS=2
RUN CARGO_BUILD_JOBS="$CARGO_BUILD_JOBS" cargo build --locked --release -p sinan-panel

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 sinan \
    && useradd --uid 10001 --gid 10001 --no-log-init --no-create-home --shell /usr/sbin/nologin sinan \
    && mkdir -p /data/artifacts \
    && chown -R 10001:10001 /data
COPY --from=builder /src/target/release/sinan-panel /usr/local/bin/sinan-panel
USER 10001:10001
WORKDIR /data
ENV SINAN_LISTEN=0.0.0.0:8080 \
    SINAN_DATA_DIR=/data \
    RUST_LOG=info
EXPOSE 8080
STOPSIGNAL SIGINT
HEALTHCHECK --interval=5s --timeout=3s --start-period=10s --retries=12 \
    CMD curl --fail --silent http://127.0.0.1:8080/healthz || exit 1
ENTRYPOINT ["/usr/local/bin/sinan-panel"]
