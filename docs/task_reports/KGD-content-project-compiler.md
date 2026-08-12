# Content-to-project compiler wave

Implemented the metadata-only adapter bridge from `kg_ddlc_plus` catalogs to
the title-neutral `keygen.project.v1` manifest.

## Delivered

- `import::project::compile_project_manifest` validates asset provenance,
  content closure, and locale fallbacks before emitting a generic project.
- Logical asset IDs, relative blob paths, and output SHA-256 values are carried
  forward; source bytes are never read by this compiler.
- Root scene declarations and story labels are derived only from the supplied
  metadata graph.
- `kg-ddlc-plus compile-project --metadata INPUT --output PROJECT` provides a
  deterministic CLI path for a player-owned metadata file.
- Synthetic coverage proves the output is generic and byte-free.

## Boundary

This wave intentionally does not discover, decode, or commit recovered game
content. A future private import job can produce the metadata input file from
the operator's local source installation.

## Verification

`cargo test -p kg-ddlc-plus`, `cargo clippy -p kg-ddlc-plus --all-targets -- -D
warnings`, and `git diff --check` pass.
