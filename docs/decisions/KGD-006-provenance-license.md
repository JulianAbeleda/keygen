# KGD-006 — provenance and local-packaging decision

Status: accepted for local macOS development; distribution review remains
required before any public or third-party package is produced.

## Decision

`kg_ddlc_plus` is an independent KeyGen compatibility target. The repository
contains importer logic, schemas, fingerprints, redacted evidence, and
synthetic tests only. A player-owned DDLC Plus installation or private
recovery is an operator-provided input and is never copied into Git.

The compiler may, subject to the operator's rights and applicable terms:

- read the selected installation or recovery without modifying it;
- reuse recovered bytes in a private, local compiled package;
- translate serialized metadata into KeyGen-owned schemas;
- reimplement runtime behavior independently.

The compiler must not:

- embed the original executable, Unity runtime, recovered/decompiled C#,
  source tree, or proprietary payload in the repository;
- emit absolute host paths in manifests or reports;
- publish a package containing recovered assets;
- overwrite the original installation or save directory;
- claim that a local compatibility build is an official release.

## Provenance fields

Every local package artifact records a logical source ID, source hash, import
mode (`copy`, `translate`, or `reimplement`), output hash, and importer version.
Reports use build IDs and redacted logical paths; absolute paths and payload
bytes are forbidden.

## Unresolved terms and release gate

This record is not legal advice and does not determine the rights of a specific
operator or jurisdiction. Steam, publisher, and third-party asset terms must be
reviewed before distribution. Until that review is documented, automation is
limited to local development from the operator's own installation and must
fail closed for publication/export commands.

## Evidence

Observation procedure: [private observation protocol](../../scripts/private/observation-protocol.md).
The repository scanner checks tracked files and synthetic fixtures only; it
does not inspect a user's installation by default.
