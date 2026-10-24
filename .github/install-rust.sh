#!/usr/bin/env bash
# Install and select the pinned Rust toolchain, then print what it actually is.
#
# Tracked as a script rather than inlined in the workflow because CI runs the
# complete lane as two jobs, and a seven-line provisioning block copied into
# both is a pin that can drift from itself. The version is read from
# `rust-toolchain.toml`, the one place it is written; tests hold the workspace's
# declared minimum equal to it, and Renovate bumps both together (renovate.json).
#
# The versions are echoed on purpose: a toolchain that installed but did not
# become the default is a job that proves a different compiler than it claims.
set -euo pipefail

RUST_TOOLCHAIN_VERSION="$(python3 -c 'import sys, tomllib; print(tomllib.load(open(sys.argv[1], "rb"))["toolchain"]["channel"])' rust-toolchain.toml)"
: "${RUST_TOOLCHAIN_VERSION:?rust-toolchain.toml must name the pinned toolchain}"

rustup toolchain install "${RUST_TOOLCHAIN_VERSION}" \
  --profile minimal --component rustfmt --component clippy --no-self-update
rustup default "${RUST_TOOLCHAIN_VERSION}"

rustc --version
cargo --version
cargo fmt --version
cargo clippy --version
