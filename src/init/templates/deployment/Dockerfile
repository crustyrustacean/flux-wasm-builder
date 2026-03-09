# Stage 1: Chef - prepare dependency caching
FROM ghcr.io/lukemathwalker/cargo-chef:latest-rust-1.85 AS chef
WORKDIR /app
RUN rustup target add wasm32-unknown-unknown && \
    curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Stage 2: Planner - analyze dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder - compile dependencies then app
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features embed-assets --recipe-path recipe.json

COPY . .
RUN wasm-pack build --release frontend && \
    grass frontend/styles/screen.scss frontend/styles/screen.css && \
    cargo build --release --features embed-assets -p {{ name }}-backend

# Stage 4: Runtime - minimal image
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/{{ name }}-backend /usr/local/bin/app
ENV APP_ENVIRONMENT=production
ENV APP_APPLICATION__HOST=0.0.0.0
EXPOSE 3001
CMD ["app"]
