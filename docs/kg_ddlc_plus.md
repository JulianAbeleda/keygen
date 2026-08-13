# `kg_ddlc_plus` compatibility target

`kg_ddlc_plus` is the KeyGen package name for the local DDLC Plus compatibility
work. The separate name prevents its output from being confused with the
original game or an official build. The compiler is open source; the input and
compiled content remain local and player-owned.

The canonical implementation contract is the
[macOS compatibility scope](kg_ddlc_plus_scope.md), and the delegation-sized
work breakdown is the [execution ledger](kg_ddlc_plus_tasks.md). This file is
only the operator overview. The method and lessons from reconstructing the
first two interactive menu surfaces are recorded in the
[two-screen first-principles retrospective](task_reports/two-screen-first-principles-learnings.md).

## Supported source

The current importer accepts one exact recovery:

- Steam app `1388880`;
- Steam build `10766092`;
- Unity editor version recorded by the source: `2019.4.20f1`;
- AssetRipper `ExportedProject` directory layout;
- exact SHA-256 fingerprints for the six files used to identify and compile
  the first boot slice.

The compiler fails if a required file is absent or differs. This is a narrow,
testable compatibility claim, not a claim that KeyGen can compile arbitrary
Unity projects.

By default the command discovers the recovery previously created at
`$HOME/ddlc-architecture-explorer/unpacked/assetripper-build-10766092/ExportedProject`.
Use `--source PATH` or `KG_DDLC_PLUS_SOURCE` on another machine.

## Commands

Run these from the KeyGen repository:

```sh
# Verify the source recovery without writing output.
cargo run -p kg-ddlc-plus -- inspect

# Compile local/kg_ddlc_plus/package.json and its first scene.
cargo run -p kg-ddlc-plus -- compile

# Validate every packaged artifact and load the entry scene.
cargo run -p kg-ddlc-plus -- validate

# Exercise the deterministic headless renderer.
cargo run -p kg-ddlc-plus -- render \
  --output local/kg_ddlc_plus/bios.png --time 4.25

# Present the same package in the native KeyGen window.
cargo run -p kg-ddlc-plus -- run
```

`--output DIR` changes the compile destination. `--package DIR` selects an
existing package for validation, rendering, or execution. All examples write
under `local/`, which Git ignores.

## What the current compiler does

The phase-zero compiler performs a real source-to-package pass:

1. fingerprints the supported source recovery;
2. parses the recovered timed BIOS text format;
3. translates timing, font, image, and geometry into `keygen.scene.v1`;
4. copies only the two assets needed by the entry scene;
5. records input and artifact SHA-256 values in `keygen.package.v1`;
6. reloads the result through `keygen-player` before reporting success.

This reaches a reconstructed BIOS scene. It does not yet reproduce the full
launcher, DDLC title menu, story interpreter, save system, or audio graph.

## Coverage phases

| Phase | Compatibility surface | Acceptance evidence |
| --- | --- | --- |
| 0 | Source identity, package manifest, BIOS text/image | compile, validate, deterministic PNG, native smoke |
| 1 | Boot log, skip behavior, boot-to-desktop state | timeline and transition fixtures |
| 2 | MES desktop and launcher navigation | focus, pointer, window-stack state tests |
| 3 | DDLC title screen and settings | visual captures and action parity |
| 4 | Visual-novel command IR | documented command subset and rejection tests |
| 5 | Dialogue, sprites, audio, saves, transitions | deterministic replay and persistence tests |
| 6 | Supported-story coverage and native packaging | clean-machine Apple Silicon macOS qualification |

Each phase extends general KeyGen primitives only when the primitive is useful
beyond this target. Product-specific mappings stay in `kg-ddlc-plus`.

## Reuse-first rule

The compatibility compiler must use recovered, player-owned images, sprites,
fonts, audio, animations, localization variants, and serialized layout values
when they exist. It may translate Unity-specific metadata into KeyGen schemas,
but it may not replace available art with generated, redrawn, stock, or
approximate content. Engine behavior such as Unity shaders and C# execution is
implemented independently in Rust and verified against private reference
captures.

## Content boundary

Never commit the recovered project, extracted source, fonts, images, compiled
package, or local absolute paths. KeyGen stores only importer logic, source
fingerprints, schemas, documentation, and synthetic tests. Compatibility does
not grant redistribution rights, and a user must supply their own installed
copy and recovery.
