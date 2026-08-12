#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

git diff --cached --check
python3 sz.py
python3 scripts/check-canonical-apps.py
python3 scripts/check-module-boundaries.py
python3 scripts/check-generic-boundary.py
cargo fmt --all -- --check
cargo check --workspace
