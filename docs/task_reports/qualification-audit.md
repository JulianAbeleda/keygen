# Qualification audit — canonical app boundary

## Scope

Audited the current `kg_ddlc_plus` package and qualification paths against
`docs/kg_ddlc_plus_tasks.md`, with emphasis on CLI/package paths, canonical
asset/package reuse, stale duplicate app bundles, and report/checker coverage.

## Findings and change

- The CLI exposes `inspect`, `compile`, `validate`, `render`, and `run`; the
  macOS packaging scripts consume the compiled arm64 binary and an optional
  imported package directory without generating replacement assets.
- The package and app identity are centralized in the existing identity and
  manifest checks. No alternate tracked app bundle or proprietary asset path
  was found.
- Added `scripts/check-canonical-apps.py`. It is non-destructive, passes when
  no local distribution exists, and rejects any `dist/macos` state containing
  stale/duplicate `.app` siblings or an incomplete canonical bundle. This
  addresses the local duplicate-search problem without deleting user files.
- Wired the check into `scripts/check-fast.sh`; the scope checker continues to
  validate all 147 packet IDs, dependency acyclicity, and report references.

## Acceptance

```text
python3 scripts/check-kg-scope.py                         PASS (147 packets)
python3 scripts/check-canonical-apps.py                   PASS (no distribution)
scripts/check-fast.sh                                     PASS
git diff --check                                           PASS
```

The generated distribution directory was absent during this audit, so the
positive duplicate/incomplete-bundle failure branches remain exercised by the
script's explicit structural checks and should be run on the Apple Silicon
packaging host after producing `dist/macos/kg_ddlc_plus.app`.

## Limitations

This audit does not import private DDLC content, infer missing source values,
or add native rendering/audio/signing adapters. Those remain the explicit
follow-up boundaries in the wave reports and scope ledger.
