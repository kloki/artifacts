FROM rust:1-slim AS builder
WORKDIR /build

# Build dependencies against a stub binary first so this layer stays cached
# across source-only changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
    && echo 'fn main() {}' > src/main.rs \
    && echo '' > src/lib.rs \
    && cargo build --release \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs src/lib.rs && cargo build --release

FROM debian:bookworm-slim AS runtime
# curl is present solely for the container healthcheck below.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --uid 10001 --create-home app \
    && mkdir -p /data \
    && chown app:app /data

COPY --from=builder /build/target/release/artifacts /usr/local/bin/artifacts

USER app
WORKDIR /home/app
VOLUME ["/data"]
EXPOSE 8080

ENV DATA_DIR=/data \
    BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://localhost:8080/healthz || exit 1

CMD ["artifacts"]
