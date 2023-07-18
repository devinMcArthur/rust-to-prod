FROM lukemathwalker/cargo-chef:latest-rust-1.70.0 AS chef

# Let's switch out working directory to `app` (equivalent to `cd app`)
# The `app` folder will be created for us by Docker in case it does not
# exist already
WORKDIR /app

# Install the required system dependencies for our linking configuration
RUN apt update && apt install lld clang -y

# Recipe creation
FROM chef AS planner
COPY . .
# Compute the lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage
FROM chef AS builder

COPY --from=planner /app/recipe.json recipe.json

# Build our project dependencies, not our application
RUN cargo chef cook --release --recipe-path recipe.json

# Up to this point, if our dependency tree stays the same,
# all layers should be cached

# Copy all files from our working environment to our Docker image
COPY . .

ENV SQLX_OFFLINE true

# Let's build our binary!
# We'll use the release profile to make it fast
RUN cargo build --release --bin rust-to-prod

# Runtime stage - use bare operating system
# The end image size is everything in `runtime` step
FROM debian:bullseye-slim AS runtime

WORKDIR /app

# Install OpenSSL - it is dynamically linked by some of our dependencies
# Install ca-certificates - it is need to verify TLS certificates
# when establishing HTTPS connections
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl ca-certificates \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder environment
# to our runtime environment
COPY --from=builder /app/target/release/rust-to-prod rust-to-prod

# We need the configuration file at runtime
COPY configuration configuration

ENV APP_ENVIRONMENT production

# When `docker run` is executed, launch the binary
ENTRYPOINT ["./rust-to-prod"]
