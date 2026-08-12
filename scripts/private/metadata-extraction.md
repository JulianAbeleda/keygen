# Private metadata extraction

`extract-project-metadata.py` is an operator-only bridge from a local
AssetRipper `ExportedProject` to the metadata JSON consumed by
`kg-ddlc-plus compile-project`. It hashes and classifies image, audio, font,
and text files in place. It does not copy, decode, or embed source bytes;
`metadata-only/<sha256>` paths are placeholders and are not playable assets.

Run it only against a player-owned export and keep the output outside Git:

```sh
python3 scripts/private/extract-project-metadata.py \
  /path/to/ExportedProject --output /tmp/kg-ddlc-plus-metadata.json
cargo run -p kg-ddlc-plus -- compile-project \
  --metadata /tmp/kg-ddlc-plus-metadata.json --output /tmp/kg-ddlc-plus-project.json
```

Traversal is bounded by 100,000 files and 20 GiB by default; use lower limits
for diagnostics. Symlinks and Unity `Library`, `Temp`, and `.git` directories
are skipped. The synthetic check creates no repository files:

```sh
python3 scripts/private/extract-project-metadata.py --self-test
```
