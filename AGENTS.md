# KeyGen contributor guide

KeyGen is an independent, clean-room Rust game engine.

## Invariants

- Headless validation, tests, and rendering cannot require an editor, account,
  activation service, display server, browser, WebView, HTTP server, or Unity.
- The deterministic composition core receives data and produces pixels without
  reading files, opening windows, or performing network activity.
- Host I/O stays in loader/player/importer boundaries.
- Source importers target documented, tested subsets and reject unsupported
  behavior explicitly.
- Never commit third-party game assets, proprietary engine code, recovered
  source, local absolute paths, or private compatibility fixtures.
- macOS, Linux, and Windows are first-class targets.
- New schemas are versioned and fail closed on unknown fields.

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --
-D warnings`, and `cargo test --workspace` before committing.
