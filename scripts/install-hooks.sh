#!/bin/sh
set -eu

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

git config core.hooksPath .githooks
echo "Installed repository hooks from .githooks"
echo "  commit-msg: bracketed KeyGen area prefix"
echo "  pre-commit: scripts/check-fast.sh"
echo "  pre-push:   scripts/check-full.sh"
