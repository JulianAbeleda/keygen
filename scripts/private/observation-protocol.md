# Private recovery observation protocol

This protocol records observations without copying recovered content into the
repository. It is for a player-owned installation or a private recovery tree.

## Rules

- Work on a copy or read-only mount where practical.
- Do not write to the original installation or its save directory.
- Record logical IDs, versions, dimensions, timings, and hashes only.
- Never paste source text, screenshots, audio, asset bytes, absolute paths, or
  decompiled implementation into Git.
- Redact host paths before moving a report into `docs/evidence/`.

## Run record

```text
record_id: OBS-____
product: kg_ddlc_plus
source_kind: Steam installation | validated private recovery
build_id: ____
engine_version: ____
locale: ____
display: ____
operator: ____
started_utc: ____
ended_utc: ____
```

## Observation checklist

1. Verify the source build and architecture without changing files.
2. Record the logical entry state and the user action sequence.
3. For each observed state, record semantic IDs, timing, dimensions, and
   SHA-256 hashes of locally retained evidence. Do not retain payloads here.
4. Record whether the value is copied, translated, reimplemented, excluded,
   or blocked, with a reason.
5. Repeat the observation once and compare the redacted records.
6. Run the private-content scanner before committing any public summary.

## Redacted state row

```text
state_id: ____
logical_source_id: ____
action_from_previous: ____
observed_at_ms: ____
source_hash: ____
output_hash: ____
import_mode: copy | translate | reimplement | excluded | blocked
dimensions_or_metrics: ____
notes_without_content: ____
```
