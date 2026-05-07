FROM rust:bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY templates/ templates/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock* ./
COPY src/ src/
COPY templates/ templates/
RUN cargo build --release

# OGP auto-generation stage
FROM python:3.12-slim AS ogp
RUN apt-get update && apt-get install -y fonts-noto-cjk && rm -rf /var/lib/apt/lists/*
RUN pip install --no-cache-dir Pillow
WORKDIR /app
COPY scripts/generate_ogp.py ./
COPY public/blog/images/ public/blog/images/
COPY content/blog/ content/blog/
ENV BLOG_DIR=content/blog IMG_DIR=public/blog/images
RUN python3 generate_ogp.py

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/yukihamada-jp ./server
COPY public/ public/
COPY --from=ogp /app/public/blog/images/ public/blog/images/
COPY content/ content/
COPY templates/ templates/
RUN mkdir -p /data

EXPOSE 8080
CMD ["./server"]
