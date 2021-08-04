ARG RUST_IMAGE=rust:1.54

FROM $RUST_IMAGE AS install-system-packages

RUN apt-get update -y --no-install-recommends \
    && \
    apt-get install -y \
        # Build automation and generation tool.
        make \
        # Programming tool for memory debugging and profilling.
        valgrind

FROM install-system-packages AS create-application-user

ARG USER=dev
RUN useradd $USER --shell /bin/bash --create-home
USER $USER

FROM create-application-user AS install-rust-tools

RUN rustup component add \
        rust-src \
        rustfmt \
        rust-docs \
        clippy \
    && \
    cargo install \
        cargo-edit \
        cargo-expand \
        cargo-valgrind \
        cargo-audit

FROM install-rust-tools AS development-environment
