FROM rust:1-slim-bookworm AS builder

WORKDIR /build

RUN rustup target add wasm32-unknown-unknown

COPY Cargo.toml Cargo.lock ./
COPY assets ./assets
COPY scripts ./scripts
COPY src ./src
COPY web ./web

RUN sh scripts/build_web.sh && \
    cargo build --release --bin trek

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends tini && \
    rm -rf /var/lib/apt/lists/* && \
    useradd --system --no-create-home --home-dir /app --shell /usr/sbin/nologin trek

WORKDIR /app

COPY --from=builder /build/target/release/trek /usr/local/bin/trek
COPY --from=builder /build/web /app/web

USER trek

ARG PORT=8080
ENV HOST=0.0.0.0
ENV PORT=${PORT}

EXPOSE ${PORT}

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/trek"]
