FROM rust:1.84-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
# Create dummy main to cache deps
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src

COPY . .
# Force recompile with real source
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/yukihamada-jp ./
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/static ./static

ENV PORT=8080
ENV HOST=0.0.0.0
EXPOSE 8080

CMD ["./yukihamada-jp"]
