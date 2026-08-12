#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

scripts/check-fast.sh
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
