ARG RUST_VERSION=1.59.0
ARG CARGO_CHEF_VERSION=latest
ARG CARGO_CHEF_IMAGE=lukemathwalker/cargo-chef:${CARGO_CHEF_VERSION}-rust-${RUST_VERSION}

FROM ${CARGO_CHEF_IMAGE} as chef
WORKDIR /app
RUN apt-get update && apt-get install lld clang -y

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
# Build the project and strip debug symbols
RUN cargo build --release --bin newsletter \
    && \
    strip target/release/newsletter

FROM debian:buster-slim AS runtime
WORKDIR /app
# Install runtime dependencies
RUN apt-get update -y && apt-get install -y --no-install-recommends openssl \
    && \
    # Clean up
    apt-get autoremove -y && apt-get clean -y && rm -rf /var/lib/apt/lists/*
# Copy binary and config directory
COPY --from=builder /app/target/release/newsletter newsletter
COPY config config
# Start newsletter service in prod environment
ENV APP_ENVIRONMENT production
ENTRYPOINT ["./newsletter"]

