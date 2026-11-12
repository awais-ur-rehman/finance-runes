#!/bin/sh
# The verification gate from AGENTS.md, in one command.
#
# Run from anywhere in the repository. On a host where Smart App Control blocks
# freshly compiled binaries, run it through the container described in
# docs/engineering/coding-standards.md instead.
set -e

cd "$(dirname "$0")/.."

echo "cargo fmt --all --check"
cargo fmt --all --check

echo "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

echo "cargo test --workspace"
cargo test --workspace
