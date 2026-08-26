#!/usr/bin/env bash
# Install and select the pinned Rust toolchain, then print what it actually is.
#
# Tracked as a script rather than inlined in the workflow because CI runs the
# complete lane as two jobs, and a seven-line provisioning block copied into
# both is a pin that can drift from itself. `RUST_TOOLCHAIN_VERSION` comes from
# the workflow's env block; tests hold it equal to `rust-toolchain.toml` and the
# workspace's declared minimum.
#
# The versions are echoed on purpose: a toolchain that installed but did not
# become the default is a job that proves a different compiler than it claims.
set -euo pipefail

: "${RUST_TOOLCHAIN_VERSION:?RUST_TOOLCHAIN_VERSION must name the pinned toolchain}"

rustup toolchain install "${RUST_TOOLCHAIN_VERSION}" \
  --profile minimal --component rustfmt --component clippy --no-self-update
rustup default "${RUST_TOOLCHAIN_VERSION}"

rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
