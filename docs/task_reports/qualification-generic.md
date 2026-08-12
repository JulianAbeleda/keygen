# Generic KeyGen qualification

`scripts/qualify-keygen.sh` is the content-free end-to-end gate for the
title-neutral substrate. It validates and inspects the sample
`keygen.project.v1`, renders its entry scene twice at the same timestamp, and
requires byte-identical PNG output. On an Apple Silicon macOS host it also
builds a temporary generic `keygen.app` with the generic bundle builder and
runs the existing arm64 bundle smoke test. Generated files are isolated in a
temporary directory and are not added to the repository.

This deliberately uses the synthetic sample project only. It proves project
loading, scene rendering, deterministic replay of the render boundary, and
generic packaging without embedding or inventing any DDLC content. The
`kg_ddlc_plus` adapter remains a separate package boundary and can consume a
local player-owned package when available.

Acceptance:

```text
scripts/qualify-keygen.sh                         PASS
python3 scripts/check-kg-scope.py                 PASS
python3 scripts/check-generic-boundary.py         PASS
cargo test --workspace --all-targets              PASS
cargo clippy --workspace --all-targets -- -D warnings PASS
```
