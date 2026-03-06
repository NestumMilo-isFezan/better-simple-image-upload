# Stage 1: Plan
FROM rust:1.85-slim AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Prepare
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer is cached as long as Cargo.toml stays the same
RUN cargo chef cook --release --recipe-path recipe.json

# Now copy the actual source and build the real binary
COPY . .
RUN cargo build --release

# Stage 4: Runtime (Tiny image)
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/image-service /app/image-service
RUN mkdir -p /app/storage/uploads
EXPOSE 3001
CMD ["./image-service"]
