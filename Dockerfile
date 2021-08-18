ARG RUST_VERSION=1.54.0
ARG CARGO_CHEF_VERSION=latest
ARG CARGO_CHEF_IMAGE=lukemathwalker/cargo-chef:${CARGO_CHEF_VERSION}-rust-${RUST_VERSION}

FROM ${CARGO_CHEF_IMAGE} as planner

WORKDIR /app
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM ${CARGO_CHEF_IMAGE} as cacher

WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:${RUST_VERSION} AS builder

WORKDIR /app
# Copy over the cached dependencies
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
COPY . .

ENV SQLX_OFFLINE true
RUN cargo build --release --bin newsletter

FROM debian:buster-slim AS runtime

WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/newsletter newsletter
COPY config config

ENV APP_ENVIRONMENT container
ENTRYPOINT ["./newsletter"]
