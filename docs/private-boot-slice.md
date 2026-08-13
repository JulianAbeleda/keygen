# Private boot-slice operator workflow

`build-boot-slice.py` is the bounded bridge from an operator-owned
AssetRipper `ExportedProject` to a locally playable KeyGen package. It selects
only the BIOS text log, DOS font, and BIOS logo by logical relative path,
copies them by verified SHA-256 into `/tmp`, and emits a title-neutral
`keygen.project.v1` manifest, scene, and `keygen.story.v1` program.

Run it with an explicit private source:

```sh
python3 scripts/private/build-boot-slice.py \
  --source /path/to/ExportedProject \
  --output /tmp/keygen-private-boot
```

The operator workflow runs `keygen validate` and `keygen render` as its final
gates and checks that the imported story has a non-empty bounded command list.
The rendered PNG, copied bytes, story text, and manifest remain under `/tmp`;
none are repository content. A missing BIOS log is a hard failure rather than
an inferred or synthetic substitute.

This is intentionally a boot slice, not a claim of full game coverage. It is
the smallest coherent end-to-end proof that private recovery, content-addressed
materialization, generic project loading, scene rendering, and story import can
work together without hardcoding the engine to a particular title.
