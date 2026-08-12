#!/bin/sh
set -eu

# End-to-end, content-free qualification for the generic KeyGen substrate.
# All generated files live in a temporary directory; this does not modify the
# checked-in sample output or the repository distribution directory.
repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/keygen-qualify.XXXXXX")
trap 'rm -rf "$tmp"' EXIT INT TERM

sample="$repo_root/examples/sample_project"
scene="$sample/scenes/start.json"
project="$sample/project.json"
first="$tmp/first.png"
second="$tmp/second.png"

cargo build -q -p keygen --release
cargo run -q -p keygen -- validate "$project"
cargo run -q -p keygen -- inspect "$project" >"$tmp/inspect.txt"
grep -q 'sample.project' "$tmp/inspect.txt"
cargo run -q -p keygen -- render "$project" --scene "$scene" --output "$first" --time 0
cargo run -q -p keygen -- render "$project" --scene "$scene" --output "$second" --time 0
cmp "$first" "$second"

if [ "$(uname -s)" = Darwin ] && [ "$(uname -m)" = arm64 ]; then
  scripts/package-keygen-macos.sh \
    --binary target/release/keygen \
    --target keygen \
    --display-name KeyGen \
    --bundle-id com.julian.keygen \
    --resources "$sample" \
    --out "$tmp/dist"
  scripts/smoke-keygen-app.sh "$tmp/dist/keygen.app" keygen com.julian.keygen
else
  echo "generic replay/render qualification OK (macOS arm64 bundle check skipped)"
fi

echo "generic KeyGen qualification OK"
