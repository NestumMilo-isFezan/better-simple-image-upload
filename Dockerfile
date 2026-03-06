# Stage 1: Plan
FROM lukemathwalker/cargo-chef:latest-rust-latest AS chef
WORKDIR /app

# Stage 2: Prepare
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Copy migration crate fully so cargo-chef can compile it to cache deps
COPY migration/ migration/
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
