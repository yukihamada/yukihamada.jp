# Use stable Rust base; rust-toolchain.toml installs the pinned nightly automatically.
FROM rust:bookworm AS builder

RUN cargo install cargo-leptos@0.3.5

WORKDIR /app
# Copy toolchain file first so rustup installs it before the full build
COPY rust-toolchain.toml .
RUN rustup show
COPY . .
RUN cargo leptos build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/yukihamada-server ./server
COPY --from=builder /app/target/site ./site

ENV LEPTOS_OUTPUT_NAME="yukihamada-jp"
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_ENV="PROD"

EXPOSE 8080
CMD ["./server"]
