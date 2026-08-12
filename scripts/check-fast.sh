#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

git diff --cached --check
python3 sz.py
cargo fmt --all -- --check
cargo check --workspace
