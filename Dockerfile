ARG RUST_DEV_ENV_IMAGE
FROM ${RUST_DEV_ENV_IMAGE}

ARG SQLX_CLI_VERSION
RUN cargo install sqlx-cli \
        --version=${SQLX_CLI_VERSION} \
        --no-default-features\
        --features postgres
