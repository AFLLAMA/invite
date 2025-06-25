# ----- CHEF STAGE (BASE) -----
# Using cargo-chef for optimized builds with dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# ----- PLANNER STAGE -----
# Creating a recipe.json file for dependency caching
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ----- BUILDER STAGE -----
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin invite

# ----- RUNTIME STAGE -----
# Slim base image for production
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# This adds libpq.so.5 which is required by Diesel ORM
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy only what's needed for the wedding invitation app
COPY --from=builder /app/target/release/invite /usr/local/bin
COPY --from=builder /app/templates /app/templates

# Setting up for fly.io deployment
EXPOSE 8080

# Launch the wedding invitation service
ENTRYPOINT ["/usr/local/bin/invite"]
