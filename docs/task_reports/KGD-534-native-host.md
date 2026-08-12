# KGD-534 — Qualified native host boundary

The native window entrypoint now performs an explicit runtime qualification
check. The supported product target is Apple Silicon macOS (`macOS arm64`);
other targets can still use `--validate` and `--render` for deterministic
headless checks, but cannot accidentally present an unqualified interactive
window.

`player::native::compile_target` records the compile-time contract and
`runtime_target` records the process host. `require_supported_host` gates the
minifb presentation edge. This is a host-policy boundary, not a claim that the
current backend is AppKit or Metal; those adapters remain separate future work.
