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
- The reusable KeyGen engine keeps macOS, Linux, and Windows source health, but
  the kg_ddlc_plus compatibility product currently targets Apple Silicon macOS
  only. Do not add Intel, universal, Linux, or Windows product work unless the
  user expands scope.
- kg_ddlc_plus is asset-reuse-first. When the player-owned recovery contains an
  image, sprite, font, audio clip, animation, localization variant, or
  serialized layout value, import that source instead of redrawing,
  AI-generating, substituting, or eyeballing it.
- Recovered assets and generated packages remain local and ignored. Git may
  contain importer logic, source fingerprints, schemas, aggregate evidence,
  original/synthetic tests, and documentation, but never proprietary assets or
  recovered source.
- New schemas are versioned and fail closed on unknown fields.

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --
-D warnings`, and `cargo test --workspace` before committing.
