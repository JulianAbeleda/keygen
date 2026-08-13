# KeyGen end-to-end completion scope

This document is the execution contract for the remaining practical goal:
launch a generic Apple-Silicon macOS KeyGen package, render its boot scene,
navigate a manifest-defined launcher route, execute imported story content, and
close cleanly. It does not expand the goal into a Unity editor replacement.

## Completion definition

The goal is complete only when one bounded qualification command proves all of
the following against a package generated below `/tmp`:

1. `keygen.project.v1` validates with explicit scenes, routes, assets, story,
   and persistence metadata.
2. Finder-style no-argument launch discovers `Contents/Resources/package` and
   enters the boot scene.
3. Boot advances to the launcher without title-specific constants.
4. A launcher activation resolves a declared route, loads its scene, and
   changes the visible host state.
5. `keygen.story.v1` loads from the package and produces a visible dialogue or
   choice boundary.
6. Dialogue advancement and choice selection are deterministic and bounded.
7. Back/escape returns to the launcher; exit reaches `Closed` after save flush.
8. The app process exits under a bounded smoke timeout with no panic/traceback.
9. The repository contains no recovered source bytes, private package files,
   host absolute paths, or unqualified capability claims.

## Work packets

### Packet A — scene materialization

Owner: player/package boundary. Dependencies: existing `ProjectManifest`,
`ProjectRouteNavigator`, `load_scene`.

Implement a title-neutral resolver from a validated route's `scene` ID to a
package-relative scene document. The resolver must reject any directory
component or traversal in the declared mapping (no basename sanitization),
missing files, duplicate/ambiguous aliases, and viewport mismatches. The native loop
must replace its current log-only route activation with load-and-swap behavior.
Legacy `--scene` bundles without a project manifest retain their existing
behavior.

Acceptance:

- synthetic package resolves two routes to two different renderable scenes;
- unknown route, missing scene, and path escape fail closed;
- route activation changes the rendered scene identity;
- `cargo test -p keygen-player`, clippy, and diff checks pass.

### Packet B — story execution and visible host output

Owner: story/player boundary. Dependencies: existing `Program`, `Vm`,
`StorySession`, `SessionState`, and package `story.json`.

Add a package-relative story loader and a host-facing execution adapter. It
must expose typed outputs (`Dialogue`, `Choice`, `Complete`) and consume
activation/advance/select/back events without reading platform state. Bound
commands, text length, choice count, and execution steps. The native host may
render a minimal dialogue/choice overlay using existing scene primitives; it
must not invent title-specific story rules.

Acceptance:

- synthetic story with dialogue → choice → two labels round-trips through the
  adapter;
- malformed schema, missing entry, oversized text, and step exhaustion fail
  closed;
- state revision/save metadata changes only through the save boundary;
- `cargo test -p keygen-player -p kg-ddlc-plus` and clippy pass.

### Packet C — package generation and content reachability

Owner: private operator scripts. Dependencies: metadata extractor and full
package builder.

Ensure every route references a renderable scene document and every scene's
asset IDs exist in the manifest. Keep copied source bytes only under `/tmp`.
Generate a deterministic boot scene plus category scenes, route metadata, and
bounded story content. Preserve hashes and reject source changes during copy.

Acceptance:

- full local recovery build reports stable file/byte/category counts;
- project validation, boot render, story load, and route closure pass;
- reachable report has no dangling scene/asset/story references;
- self-test works without the private recovery present.

### Packet D — native interaction qualification

Owner: macOS player/package scripts. Dependencies: A–C.

Package the arm64 binary and resources, launch without arguments, and exercise
the process with a bounded smoke harness. The harness must capture stdout and
stderr, terminate only its own child, distinguish a normal timeout from a
crash, and verify the package manifest hashes. Interactive window tests remain
host-qualified; headless tests must never require a display server.

Acceptance:

- `scripts/qualify-keygen-e2e.sh` (or equivalent) exits zero on Apple Silicon;
- app reaches boot and route/story checkpoints in a deterministic host trace;
- no panic, traceback, missing-resource error, or leaked child process;
- non-macOS hosts skip only the native launch portion with an explicit reason.

### Packet E — repository and release gates

Owner: integration pass. Dependencies: all packets.

Run formatting, clippy, workspace tests, `sz.py`, private-content scan,
generic-boundary checks, package smoke, and `git diff --check`. Review that
Metal and text-input capabilities remain false unless real adapters exist;
do not mask missing functionality with claims. Commit with an allowed subject,
push `main`, and verify a clean worktree plus remote SHA.

## Deliberate non-goals

True Metal rendering, Unity editor tooling, 3D/physics/shader systems,
signing/notarization, and a full AppKit IME client are not prerequisites for
the visual-novel runtime completion defined above. They require separate
scopes and must not block this acceptance path.

## Dependency graph

```text
schema/project validation
          ↓
 A: route → scene materialization ─┐
                                   ├─ D: native e2e qualification → E: release
 B: story loader/output ───────────┘
          ↑
 C: full package reachability
```
