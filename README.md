# KeyGen

KeyGen is a free, open-source, headless-first game engine written in Rust. Its
goal is to fill the role of a general game runtime and build tool without
requiring a proprietary editor, account, activation service, or runtime.

The current `0.1.0` vertical slice is deliberately small: it validates a
versioned 2D scene, loads PNG and TrueType/OpenType assets, deterministically
composes a frame, renders timed text, outlined menu text, and transitions,
accepts keyboard and pointer input, writes headless PNG output, and opens a
native window.

## Honest compatibility boundary

KeyGen does **not** compile arbitrary Unity projects or Unity C# assemblies.
The planned compatibility layer imports a documented subset of Unity-origin
project data into KeyGen's own schemas. Once imported, validation, rendering,
packaging, and execution are entirely KeyGen-owned and do not invoke the Unity
Editor, Unity command line, Unity Player, or Unity services. Unsupported
components will fail explicitly instead of being guessed.

```text
supported source project data
             |
             v
       importer adapters
             |
             v
     KeyGen project document
             |
        +----+----+
        |         |
        v         v
  headless PNG  native player
```

Compatibility with a file format does not grant rights to third-party games,
assets, fonts, trademarks, or code. KeyGen contains none of those materials.

## Repository boundary

- `keygen-engine` owns validated scene data, images, deterministic composition,
  text, transitions, particles, and hit testing.
- `keygen-player` owns command-line parsing, filesystem loading, the native
  window, input dispatch, and headless render output.
- `kg-ddlc-plus` is an unpublishable, private compatibility compiler. It reads
  a player-owned recovery, emits ignored local KeyGen package data, and contains
  no third-party assets or recovered source in Git.
- Games own their content, product state, terminal/AI integration, and any
  source-format import fixtures.

The engine library has no Unity, browser, WebView, JavaScript, HTTP, or local
server dependency.

## Commands

```sh
# Build and test without opening a window.
cargo test --workspace

# Validate a scene and all referenced assets.
cargo run -p keygen-player -- --scene path/to/scene.json --validate

# Render deterministically without a display server.
cargo run -p keygen-player -- \
  --scene path/to/scene.json --render frame.png --time 4.25

# Run the same composition path in a native window.
cargo run -p keygen-player -- --scene path/to/scene.json

# Compile the supported DDLC Plus recovery into ignored local package data.
cargo run -p kg-ddlc-plus -- compile

# Render or run that compiled package through KeyGen.
cargo run -p kg-ddlc-plus -- render --output local/kg_ddlc_plus/bios.png
cargo run -p kg-ddlc-plus -- run
```

The minimum supported Rust toolchain is 1.85, the first stable release with
edition-2024 Cargo manifest support required by current cross-platform window
dependencies.

Scene asset paths may be absolute or relative to the scene document. The
current schema identifier is `keygen.scene.v1`.

## Roadmap

1. Stabilize the 2D scene and deterministic frame contract.
2. Add audio, screen stacks, actions, save data, and accessible semantics.
3. Add a small data-driven behavior/story instruction set.
4. Add an asset compiler and reproducible game-package format.
5. Add native macOS, Linux, and Windows packaging.
6. Add source importers, beginning with documented Unity scene/prefab subsets.
7. Keep any visual editor optional; command-line builds remain first-class.

See [architecture.md](docs/architecture.md) and
[compatibility.md](docs/compatibility.md) for the governing boundaries. The
private target, supported source fingerprint, commands, and phased coverage are
documented in [kg_ddlc_plus.md](docs/kg_ddlc_plus.md).

## License

KeyGen is released under the [MIT License](LICENSE).
