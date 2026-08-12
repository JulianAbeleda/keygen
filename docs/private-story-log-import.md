# Private story-log import

`scripts/private/import-story-log.py` converts explicitly supplied, locally
recovered `bios.txt`/`bootlog.txt` files into a generic `keygen.story.v1`
program. It accepts timestamped lines such as `1.25: text` or `[1.25] text`;
untimestamped lines are emitted at the current clock. The output is bounded by
`--limit` (512 by default).

The output may contain proprietary or player-owned text. Write it only below
`/tmp` (or another private location), inspect it locally, and do not commit it.
The importer refuses paths inside the repository. It does not discover files,
download content, or infer product-specific identifiers.

Self-test:

```sh
python3 scripts/private/import-story-log.py --self-test
```

Example (operator-only):

```sh
python3 scripts/private/import-story-log.py /private/recovered/bios.txt \
  /private/recovered/bootlog.txt --output /tmp/keygen-private/story.json
```
