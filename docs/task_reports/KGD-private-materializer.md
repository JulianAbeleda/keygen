# Private materializer

`scripts/private/materialize-project.py` is the final operator-only bridge
between a metadata extraction and a playable, generic `keygen.project.v1`
package. It requires both an explicit metadata file and an explicit local
`ExportedProject`. Selection is bounded with `--id`, `--glob`, and `--limit`.

The command copies selected bytes into an output directory beneath `/tmp`,
verifies every source SHA-256 before copying, rewrites `project.json` with
temporary `assets/<sha256>` paths, and prunes scene references to the selected
set. It refuses repository paths and never writes `local/`, tracked files, or
absolute source paths into the output. The metadata extractor records only a
relative `source_path`, so provenance remains useful without leaking the
operator's filesystem layout.

```sh
python3 scripts/private/extract-project-metadata.py /path/to/ExportedProject \
  --output /tmp/kg-ddlc-plus-metadata.json
python3 scripts/private/materialize-project.py \
  --metadata /tmp/kg-ddlc-plus-metadata.json \
  --source /path/to/ExportedProject --glob 'source.Assets.*' \
  --limit 128 --output /tmp/kg-ddlc-plus-package
```

This does not make proprietary bytes suitable for Git. `/tmp` is an
intentional disposable working location, and operators should remove the
package when finished.
