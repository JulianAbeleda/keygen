# Architecture

## Product definition

KeyGen is both a reusable engine and a command-line/native player. The long-term
product occupies the same category as a general game engine, while remaining
headless-first and open source. An editor is an optional client of the same
project schemas, never a prerequisite for a build.

## Layering

```text
source adapters                 KeyGen-owned pipeline

Unity subset ----+              project documents
other formats ---+-> import ->  + content-addressed assets
native KeyGen ---+                    |
                                      v
                              validation / compilation
                                      |
                                      v
                               portable game package
                                      |
                         +------------+------------+
                         |                         |
                         v                         v
                  deterministic core       platform host
                  state -> frame/effects   window/input/audio/fs
```

The deterministic core is platform-neutral. It never opens files or windows.
Loaders create validated in-memory resources; the player presents the resulting
surface and routes normalized input. Future scripting produces typed effects
that a host must authorize and execute.

## Current vertical slice

`keygen-engine` implements:

- strict `keygen.scene.v1` data;
- ordered PNG layers and alpha composition;
- design-space coordinates and bilinear scaling;
- font rasterization with fill and outline;
- easing, entrances, motion, particles, and fades;
- independently timed text layers for non-interactive boot sequences;
- menu focus geometry and deterministic RGBA output.

`keygen-player` implements:

- scene-relative asset resolution and bounded filesystem loading;
- schema and asset validation;
- deterministic headless PNG rendering;
- a resizable native window;
- keyboard and pointer menu interaction.

`kg-ddlc-plus` is intentionally outside the reusable engine boundary. It
validates one known player-owned AssetRipper recovery, translates a supported
subset into KeyGen-owned schemas, and writes only to ignored local output. It
does not make DDLC Plus content part of KeyGen.

This is the first engine slice, not yet a full Unity replacement. Audio,
general scene graphs, behavior execution, asset compilation, packaging, 3D,
physics, networking, an editor, and source importers remain roadmap work.
