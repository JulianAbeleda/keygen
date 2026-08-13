# Antialiased 2D shape substrate: exhaustive execution scope

Date: 2026-08-13

## Decision summary

The visible GameTerm card-edge gap is real, bounded, and belongs primarily in
KeyGen's shape rasterizer.

It is not evidence that DDLC Plus has access to a mysterious Unity-only UI
primitive. The two applications currently take different rendering paths:

```text
GameTerm localhost
  CSS clip-path polygon
    -> browser coverage/edge antialiasing
    -> composited display pixels

GameTerm native
  configured polygon points
    -> KeyGen CPU point-in-polygon test
    -> one binary inside/outside result per physical pixel
    -> completed bitmap
    -> minifb Metal presenter with nearest sampling

DDLC Plus launcher/title surfaces
  authored PNG/sprite/atlas components + SDF text
    -> Unity textured meshes and alpha blending
    -> Retina backing
    -> project quality tier with 4x MSAA on standalone
```

The most important difference is earlier than Metal: DDLC's defining menu
surfaces are authored raster assets whose edge coverage is already represented
in image alpha, while GameTerm asks KeyGen to manufacture angled silhouettes at
runtime. Unity's Retina and MSAA configuration provides another quality layer,
but it is not the sole reason DDLC looks sharp.

The recommended implementation is a platform-neutral, antialiased path-mask
adapter in `keygen-engine`, initially used by `Canvas::polygon`. The leading
candidate is `tiny-skia`'s CPU path rasterizer because it is pure Rust,
headless, supports 8-bit antialiased masks, and preserves KeyGen's deterministic
data-to-pixels architecture. A small measured spike must first prove exact
output stability, blend semantics, memory bounds, and performance. If it fails
those gates, use the fixed-sample fallback specified below.

This is not a Metal rewrite. The focused polygon fix is a moderate engine task.
A complete Unity-class GPU/vector renderer is a separate, substantially larger
program and is not required to make these menu boxes sharp.

## What was audited

### KeyGen engine

The current implementation in `crates/engine/src/surface.rs` does the following:

1. `Canvas::new_scaled` multiplies logical dimensions by a manually supplied
   density, currently `2` for GameTerm.
2. `Canvas::polygon` multiplies each vertex by that density.
3. It scans the integer bounding box.
4. It runs an even/odd ray-crossing test once at `(x, y)` for each pixel.
5. A hit blends the color at full coverage; a miss contributes nothing.

There is no fractional pixel coverage, edge mask, multisampling, or analytic
area calculation. Density 2 creates more, smaller staircase steps; it does not
turn those steps into antialiased coverage.

The surrounding paths are not all equivalent:

- fontdue already supplies per-glyph alpha coverage;
- scaled images use `Image::sample_bilinear`;
- polygons, rounded rectangles, circles used by `dot_grid`, and transformed
  solid geometry use binary coverage;
- `Surface::blend` composites source color into an opaque RGBA8 destination
  and forces destination alpha to 255.

That last behavior is acceptable for the current opaque frame buffer, but the
new mask adapter must feed coverage into `Surface::blend` rather than treating
mask alpha as a second independent destination alpha channel.

### KeyGen macOS host

`crates/player/src/host.rs` creates a high-density CPU `Canvas`, converts it to a
packed RGB buffer, and hands the completed buffer to minifb.

The macOS minifb backend does use Metal, but only as a presenter:

- it uploads the CPU bitmap to a `BGRA8Unorm` texture;
- it draws one full-window textured quad;
- its shader explicitly uses nearest-neighbor minification and magnification;
- it does not receive KeyGen paths, vertices, coverage masks, or draw commands.

Consequently, Metal cannot repair a jagged edge already baked into the uploaded
bitmap. This also means adding MSAA only to that final textured quad would not
solve the defect: the quad's outer boundary is not the GameTerm card boundary.

The host currently accepts a manual `pixel_density` instead of reading the
window's live backing conversion. That is not the primary jagged-edge cause,
but it is a separate correctness risk when a window moves between Retina and
non-Retina displays. Apple recommends backing-coordinate conversion rather than
treating a scale factor as a global constant.

### GameTerm browser and native consumers

The canonical browser projection uses `clip-path: polygon(...)` for the paper,
menu cards, and notice surfaces in `preview/preview.css`. The browser receives
the same normalized points from the presentation contract and rasterizes their
edges through its own graphics stack.

The native consumer in GameTerm's `crates/native/src/lib.rs` reconstructs those
layers with repeated `Canvas::polygon` calls. The focused menu row is two
overlaid neutral silhouettes: a white rear polygon and a smaller/offset black
front polygon. That geometry is now structurally aligned with the localhost,
but every slanted boundary still goes through KeyGen's binary fill.

The app requests a 1600x900 logical window at density 2. At that setting the CPU
surface is 3200x1800, or 5.76 million RGBA pixels (about 23 MiB before the packed
presentation buffer). The menu normally redraws only on input/state changes,
although transitions and the active scene can redraw at 30 FPS.

### Recovered DDLC Plus evidence

The private recovered export was inspected without adding recovered source or
assets to this repository. The relevant aggregate findings are:

- the outer Plus start-menu panel is a real 872x1267 RGBA sprite;
- its focused menu row is a real 870x145 atlas crop displayed at about 436x73;
- the launcher also uses separate icon variants, shadow sprites, and taskbar
  crops rather than synthesizing those surfaces from arbitrary polygons;
- the inner DDLC title screen references authored layers including
  `gui/menu_bg.png`, `gui/overlay/main_menu.png`, character art, logo, and
  particles;
- the inner navigation frame resolves through the `menu_nav` image/transform
  path rather than a one-sample procedural polygon fill;
- the launcher contains TextMesh Pro SDF font atlases;
- recovered project settings enable macOS Retina support;
- the standalone platform defaults to the `Ultra` tier, whose recovered quality
  setting uses 4x MSAA; lower tiers range from disabled to 2x/4x.

The start-menu texture is imported with alpha transparency, no mipmaps, and a
filtered texture mode rather than point sampling. This is consistent with a UI
surface whose authored alpha and texture sampling preserve smooth edges.

These facts explain why the observed DDLC menus do not expose the same defect:

1. **Authored coverage:** important silhouettes, highlights, texture, and edge
   treatment live in sprite pixels/alpha rather than being regenerated by a
   primitive point test.
2. **Filtered textured rendering:** Unity draws those sprites as textured UI
   geometry instead of handing the display a nearest-scaled, one-bit polygon
   mask.
3. **High-resolution backing:** the recovered macOS player opts into Retina.
4. **Configured MSAA:** the default standalone quality tier requests four
   samples for geometry to which MSAA applies.
5. **SDF typography:** the Plus launcher has an independent crisp-text path.

This does not mean Unity makes every UI edge perfect automatically. MSAA
applicability depends on the rendering path, lower quality tiers can disable it,
and sprites can still blur when scaled poorly. DDLC avoids this particular
failure structurally and then benefits from Unity's mature presentation stack.

## Grounding in primary documentation

- The [CSS Masking specification](https://www.w3.org/TR/css-masking-1/)
  defines clipping through a polygon/shape and explicitly allows antialiasing
  along the geometry edge. The exact browser sampling algorithm is an
  implementation detail, so localhost is an observed visual oracle, not a
  normative pixel oracle.
- Unity's
  [`QualitySettings.antiAliasing`](https://docs.unity3d.com/2022.3/Documentation/ScriptReference/QualitySettings-antiAliasing.html)
  defines 0, 2, 4, and 8 samples per pixel for GPU MSAA.
- Unity's [`FilterMode`](https://docs.unity3d.com/2019.4/Documentation/ScriptReference/FilterMode.html)
  documents point, bilinear, and trilinear texture sampling; bilinear samples
  are averaged rather than selected as one nearest texel.
- Unity's
  [UI graphics preparation guide](https://docs.unity3d.com/6000.0/Documentation/Manual/best-practice-guides/ui-toolkit-for-advanced-unity-developers/graphic-and-font-assets-preparation.html)
  describes the common workflow of exporting UI artwork as lossless transparent
  bitmaps and packing components into atlases.
- Unity's
  [font preparation guide](https://docs.unity3d.com/6000.0/Documentation/Manual/best-practice-guides/ui-toolkit-for-advanced-unity-developers/text.html)
  explains that SDF font assets remain crisp under transformations and
  magnification.
- Apple's
  [Metal MSAA guide](https://developer.apple.com/documentation/Metal/improving-edge-rendering-quality-with-multisample-antialiasing-msaa)
  explains multisample edge rendering and resolve.
- Apple's
  [high-resolution API guide](https://developer.apple.com/library/archive/documentation/GraphicsAnimation/Conceptual/HighResolutionOSX/APIs/APIs.html)
  distinguishes logical points from backing pixels and recommends backing-store
  conversions.
- Apple's
  [Core Animation layer guide](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/CoreAnimation_guide/SettingUpLayerObjects/SettingUpLayerObjects.html)
  notes that path-based shape layers remain crisp compared with scaling a layer
  backing bitmap.
- [`tiny-skia`](https://github.com/linebender/tiny-skia) is a pure-Rust,
  CPU-only Skia subset focused on 2D rendering quality, speed, and small binary
  size. Its
  [`Pixmap::fill_path`](https://docs.rs/tiny-skia/latest/tiny_skia/struct.Pixmap.html#method.fill_path)
  and [8-bit `Mask`](https://docs.rs/tiny-skia/latest/tiny_skia/struct.Mask.html)
  expose antialiased path coverage without requiring a GPU or display.

## Desired end state

KeyGen should have one reusable shape-coverage substrate with these properties:

- platform-neutral Rust implementation;
- usable in headless validation and rendering;
- no editor, account, browser, WebView, display, or network dependency;
- deterministic RGBA8 output for the same scene, time, assets, and dimensions;
- logical coordinates with explicit physical density;
- even/odd and winding fill semantics represented explicitly;
- fractional coverage at diagonal and curved edges;
- correct source-over composition for translucent colors and overlapping
  layers;
- bounded allocation based on a clipped shape region, not a full-canvas
  temporary per primitive;
- a deliberate opt-out for pixel art or strict binary masks;
- identical headless and native pixels before host presentation;
- tests that make edge quality and performance regressions visible.

For GameTerm specifically, the 2x native frame should show the same visual
relationship as localhost—white rear card, smaller black front card, clean
diagonals—without requiring card-specific PNGs or CSS.

## Scope boundaries

### KGR-006 host/backing-scale audit (2026-08-13)

The player host was audited separately from the rasterizer. `WindowPolicy`
dimensions are logical points; `pixel_density` is the explicit logical-to-
backing conversion used to size the CPU canvas and the minifb upload. The
conversion is now exposed as a checked `physical_size()` helper and validated
before opening a window, with headless tests covering the 2x Retina case and
overflow rejection. The native adapter continues to report `Minifb` and does
not claim an AppKit/Metal renderer. Its backend-neutral `HostFrame` contract
already validates exact RGBA payload size, while `map_pointer` preserves
aspect-ratio letterboxing and remains display-independent.

The headless host now also exposes `render_frame_scaled`, which accepts logical
dimensions, an explicit density, view/frame ID, and caller-supplied time, then
returns a physical-resolution `Surface`. The existing `render_frame` delegates
to this API at density 1; invalid densities and integer-size overflow fail
closed without touching a display.

No generic evidence justified changing minifb's presentation mode: it receives
the completed CPU bitmap, not shape geometry, and its scale mode is already
explicit (`X1` plus aspect-ratio stretch). A live Retina backing conversion
cannot be obtained portably from minifb; a future AppKit host should replace
the manual density policy with the window's backing-coordinate conversion.

### In scope

- antialiased filled polygons in `keygen-engine`;
- explicit fill rule and antialias policy;
- robust input handling for non-finite, degenerate, concave, offscreen, and
  self-intersecting point sets;
- coverage-aware compositing into KeyGen's opaque surface;
- deterministic unit/golden/performance tests;
- migration of other visibly curved binary primitives after polygon acceptance;
- audit and correction of macOS backing-density/presentation scaling;
- GameTerm dependency update, canonical app rebuild, screenshot comparison, and
  managed stale-bundle cleanup;
- documentation of public behavior and migration consequences.

### Out of scope for this pass

- replacing minifb with a custom Metal renderer;
- a general Unity scene/shader/material replacement;
- 3D geometry, lighting, post-processing, or deferred rendering;
- browser pixel identity at arbitrary zoom or on every browser;
- changing GameTerm's canonical presentation geometry or two-card design;
- baking product-specific card PNGs into KeyGen;
- changing font rendering, except to ensure shape work does not regress it;
- gradients, blur kernels, arbitrary blend modes, or color-managed HDR;
- recovered DDLC assets, code, or private paths in git.

## Rendering options considered

| Option | Quality | Headless/cross-platform | Determinism | Cost/risk | Decision |
| --- | --- | --- | --- | --- | --- |
| Keep binary fill and increase density | More staircase samples, still no coverage | Yes | High | High memory, does not solve root cause | Reject |
| Bake GameTerm cards as PNGs | Can look sharp for fixed shapes | Yes | High | Product-specific, weakens dynamic configuration | Reject as engine fix |
| Full-canvas 4x supersample/downsample | Good | Yes | High with fixed filter | 16x sample work and very large temporary surfaces | Reject |
| Local fixed-grid supersampling | Good for menu polygons | Yes | High with fixed-point samples | Small code; quality/performance tradeoff is ours to maintain | Fallback |
| Exact analytic pixel/shape intersection | Excellent for simple convex polygons | Yes | High | Concave paths, self-intersection, strokes, and curves greatly increase complexity | Do not start here |
| `tiny-skia` local 8-bit coverage mask | Mature path AA; extensible to strokes/curves | Yes | Must be proven across targets | New BSD dependency and adapter work | Recommended after spike |
| Core Graphics/CAShapeLayer | Excellent on macOS | No | Host-dependent | Breaks platform-neutral deterministic core | Reject for core |
| Direct Metal/wgpu geometry + MSAA | Excellent and scalable | Requires CPU fallback | GPU-dependent | New renderer, shaders, resource lifecycle, parity path | Future program |

## Recommended architecture

### Public API

Introduce generic, title-neutral shape policy:

```rust
pub enum FillRule {
    EvenOdd,
    Winding,
}

pub enum AntialiasMode {
    None,
    Coverage,
}

pub struct FillOptions {
    pub fill_rule: FillRule,
    pub antialias: AntialiasMode,
}
```

Add `Canvas::fill_polygon_with_options(points, color, options)`. Keep
`Canvas::polygon(points, color)` as the compatibility convenience API, but make
its documented default `EvenOdd + Coverage`. `AntialiasMode::None` preserves a
pixel-art/binary-mask escape hatch.

Do not add a DDLC or GameTerm switch. Do not place a renderer policy in product
presentation JSON unless a real product needs to override the default.

### Coverage adapter

For the recommended `tiny-skia` path:

1. Reject fewer than three points, non-finite coordinates, and bounds that
   cannot be represented safely.
2. Multiply logical coordinates by `Canvas::density` exactly once.
3. Compute a conservative physical bounding box with a one-pixel antialias
   fringe and clip it to the destination surface.
4. Translate vertices into bbox-local coordinates.
5. Build a closed path.
6. Allocate one 8-bit mask sized to the clipped bbox, not the full surface.
7. Fill with the requested fill rule and antialias flag.
8. For every nonzero mask byte, call `Surface::blend` with
   `coverage / 255.0` as opacity. Source alpha remains part of the normal blend.
9. Drop the temporary mask at the end of the call.

This preserves KeyGen's existing destination format and makes coverage a
separate, testable concern. A later optimization may reuse scratch storage or
cache immutable masks, but no global cache should be introduced before a
profile proves it necessary.

### Fixed-sample fallback

If the dependency spike fails determinism, compatibility, or performance gates,
implement a bbox-local fixed sample grid:

- use an explicit 4x4 pattern at fixed subpixel offsets;
- evaluate the chosen fill rule at all 16 samples;
- coverage is the integer hit count divided by 16;
- use fixed-point vertex/sample coordinates so target SIMD and floating-point
  contraction cannot change golden pixels;
- preserve a fast opaque interior and empty exterior when profiling justifies
  the added classification logic;
- keep the same public API so the backend can change without consumer churn.

A 2x2 pattern is permitted only if screenshot and acute-angle fixtures prove it
visually sufficient. A whole-frame supersample buffer is not permitted.

### Primitive migration order

Only filled polygons are required to fix GameTerm's cards. After that gate:

1. route `rounded_rect` through the path substrate;
2. route dot/circle edges through coverage or retain a documented pixel-art
   mode;
3. introduce generic stroked paths only when a consumer needs them;
4. add quadratic/cubic curves as path features, not independent raster loops;
5. add clipping masks and gradients only as separately scoped capabilities.

Axis-aligned integer rectangles may retain their optimized fill path. Fractional
or transformed rectangles should use coverage when their edges are not
pixel-aligned.

### Host density and final presentation

Treat host scaling as a parallel verification track, not as a substitute for
shape AA:

- record logical window size, drawable/backing size, input coordinates, source
  buffer size, and final viewport in a diagnostic build;
- verify 1600x900 logical maps to 3200x1800 backing pixels on the current Retina
  display with no hidden second resample;
- verify the window library's nearest presenter is exactly 1:1 at that mapping;
- define behavior when moving between 1x and 2x displays;
- prefer live backing conversion over a permanent manual density once the
  native adapter exposes it safely;
- rerasterize at the new density rather than stretching an old bitmap;
- keep headless tests able to request density 1 and 2 explicitly.

Changing minifb's final sampler to linear may hide some scaling mistakes but is
not an acceptable polygon-AA implementation. At exact integer presentation,
nearest sampling is useful because it prevents an additional blur pass.

## Work breakdown

### KGR-001 — Baseline evidence and synthetic fixture

**Owned paths:** new engine fixture/test/report paths only.

- Add a synthetic scene containing axis-aligned edges, shallow and steep
  diagonals, acute corners, concave polygons, two offset card layers,
  translucent overlap, and density 1/2 variants.
- Capture the current binary output and pixel histograms.
- Record current release-mode polygon and full-frame timing.
- Record the current KeyGen and GameTerm revisions used by the evidence.
- Do not make the current jagged output the accepted final golden.

**Exit:** reproducible baseline images, hashes, timings, and a fixture that can
distinguish binary from partial coverage.

### KGR-002 — Raster backend decision spike

**Owned paths:** isolated prototype/benchmark and dependency manifest.

- Rasterize the fixture through a bbox-local `tiny-skia::Mask`.
- Prove RGBA channel order and source-over alpha behavior.
- Run repeated renders and compare exact hashes.
- Run on macOS arm64 and at least one non-macOS CI target.
- Measure temporary bytes, allocations, polygon time, and full-frame delta.
- Audit crate version, MSRV, transitive dependencies, license, and binary-size
  delta.
- Prototype the fixed 4x4 fallback only if a gate fails; do not land two
  production backends without a demonstrated need.

**Decision gate:** select `tiny-skia` when output is stable, quality passes, no
full-canvas allocation occurs, workspace source health passes, and performance
meets KGR-007. Otherwise record the failed gate and select fixed sampling.

### KGR-003 — Shape API and polygon integration

**Owned paths:** `crates/engine` shape/surface modules and unit tests.

- Add `FillRule`, `AntialiasMode`, and `FillOptions`.
- Add the explicit polygon API and compatibility wrapper.
- Implement conservative clipping and local mask translation.
- Make non-finite, degenerate, empty, and entirely offscreen input safe no-ops
  or typed errors according to existing Canvas conventions.
- Preserve even/odd semantics of the old polygon helper.
- Document the default-output change.

**Exit:** all filled polygon callers receive partial edge coverage by default;
binary mode remains explicitly testable.

### KGR-004 — Coverage composition correctness

**Owned paths:** `crates/engine/src/surface.rs` and focused tests.

- Test opaque and translucent colors over opaque backgrounds.
- Test two antialiased layers sharing, crossing, and nearly touching edges.
- Ensure coverage multiplies source alpha once and only once.
- Ensure no dark or light halo is introduced by straight/premultiplied alpha
  mismatch.
- Verify output alpha remains consistent with KeyGen's opaque-frame contract.

**Exit:** edge pixels match documented source-over calculations and halo
fixtures pass.

### KGR-005 — Remaining binary primitive audit

**Owned paths:** engine primitive code/tests.

- Classify rectangles, rounded rectangles, dot/circle rendering, image edges,
  and text by coverage behavior.
- Migrate rounded rectangles and non-pixel-art circles to the shared substrate.
- Retain optimized aligned rectangles with explicit pixel-center rules.
- Add one document explaining which primitive owns antialiasing and which owns
  sampling.

**Exit:** no common smooth-shape primitive silently uses a one-sample edge.

### KGR-006 — Native backing-scale audit

**Owned paths:** `crates/player` host diagnostics/adapter/tests and host docs.

- Add an opt-in diagnostic record for logical, backing, buffer, and viewport
  sizes.
- Test 1x and 2x requested density headlessly.
- Verify current macOS presentation experimentally at exact scale.
- Specify a safe path for live display-scale changes without introducing a
  macOS dependency into the engine core.
- Correct the host only if evidence shows a second scale or stale density.

**Exit:** the native host either proves a 1:1 Retina upload or contains a tested
  correction; the result is recorded.

### KGR-007 — Performance and allocation qualification

**Owned paths:** benchmark/qualification scripts and report.

- Benchmark release builds after warm-up.
- Measure p50/p95 for the synthetic polygon batch and GameTerm-equivalent boot
  frame.
- Set the final regression threshold from the KGR-001 baseline.
- Hard gate: a 30 FPS animated consumer must remain below 33.3 ms p95 for a
  full frame on the reference Apple Silicon machine.
- Target: antialiasing adds no more than 20% to the measured full boot-frame
  raster time unless the absolute frame remains comfortably under budget and
  the exception is documented.
- Hard gate: no temporary full-canvas RGBA supersample per polygon.
- Report peak bbox mask memory and allocation count.

**Exit:** measured quality does not create an unbounded CPU/memory regression.

### KGR-008 — GameTerm consumer rollout

**Owned paths:** GameTerm dependency lock/manifest, tests, report, and only
renderer-specific consumer code if required.

- Advance the pinned KeyGen git revision deliberately.
- Do not change presentation points, offsets, colors, or card layer count to
  conceal renderer defects.
- Render menu states: idle, each focused row, pressed, disabled, transition
  locked, and settings modal.
- Confirm pointer/hit geometry is unchanged because antialiasing is visual only.
- Rebuild the canonical macOS app through the existing package workflow.
- Inventory managed GameTerm bundles before install, atomically replace the
  canonical bundle, and remove only obsolete bundles positively identified by
  the managed build markers. Do not delete arbitrary similarly named apps.

**Exit:** the installed canonical app uses the new KeyGen revision and no
managed stale build remains searchable as a competing launch target.

### KGR-009 — Browser/native visual acceptance

**Owned paths:** comparison tooling, screenshots ignored by git where private,
and aggregate report.

- Capture localhost and native app at the same logical 1600x900 design state.
- Compare 1x display, 2x Retina, and 4x inspection crops.
- Use edge crops for the white rear and black front cards, not only whole-screen
  perceptual comparison.
- Record partial-coverage histograms and scanline profiles across representative
  diagonals.
- Treat localhost as the design oracle, not an exact RGBA oracle: browser edge
  filters need not produce identical intermediate alpha values.
- Confirm no new blur in text, background art, or axis-aligned regions.

**Exit:** human review accepts line sharpness and automated evidence proves the
native path is no longer binary.

### KGR-010 — Full release gate and documentation

**Owned paths:** public docs, changelog/release report, CI wiring.

- Run `cargo fmt --all -- --check`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.
- Run `cargo test --workspace`.
- Run cross-platform source-health checks already required by KeyGen.
- Run GameTerm's full validation and packaging checks.
- Smoke the installed macOS app and exercise focus/activation/exit.
- Document API default, performance, dependency/license decision, known
  limitations, and future GPU boundary.

**Exit:** all gates pass from a clean checkout plus permitted private assets;
the report links exact revisions and evidence.

## Test matrix

| Dimension | Required cases |
| --- | --- |
| Geometry | triangle, quad, hexagon, concave, self-intersecting, clockwise/counterclockwise, repeated point, collinear, zero area |
| Position | integer, half-pixel, arbitrary fraction, negative/offscreen, clipped at every canvas edge |
| Edge | horizontal, vertical, 1:N shallow, N:1 steep, acute, obtuse |
| Fill | opaque, translucent, source alpha 0/1/intermediate, two overlapping fills |
| Policy | even/odd, winding, coverage, none |
| Density | 1x, 2x, 3x/4x API bounds |
| Destination | light, dark, high-contrast, already composited |
| Runtime | headless PNG, native exact-scale upload, native display-scale transition |
| Consumer | all GameTerm card states, paper surface, notice, background accent polygons |

Required assertions include:

- at least one diagonal boundary pixel has fractional coverage;
- fully interior pixels remain exact source color for opaque fills;
- fully exterior pixels remain exact destination color;
- reversing a simple polygon does not alter even/odd output;
- density changes physical detail without changing logical bounds;
- repeated runs produce identical PNG bytes;
- headless and pre-presentation native surfaces hash identically at the same
  dimensions/density;
- binary mode remains binary;
- no panic or unbounded allocation occurs for rejected inputs.

## Acceptance criteria

The work is complete only when all of the following are true:

1. GameTerm's native white/black menu-card diagonals contain fractional edge
   coverage and no visible staircase at normal Retina viewing distance.
2. The native geometry, offsets, state layers, colors, and labels still come
   from the canonical presentation contract.
3. Localhost and native show the same two-shape relationship; only expected
   renderer-specific edge samples differ.
4. The engine solution is generic and contains no product name or recovered
   asset assumption.
5. Headless rendering remains independent of a display, GPU, editor, account,
   browser, and network.
6. Deterministic and cross-platform checks pass.
7. The 30 FPS full-frame performance gate passes and memory is bbox-bounded.
8. Retina presentation is proven 1:1 or corrected with a tested adapter change.
9. The canonical installed GameTerm app is rebuilt against the accepted KeyGen
   revision and managed duplicates are cleaned safely.
10. The KeyGen and GameTerm commits are pushed directly to their intended main
    branches under the solo-development workflow.

## Effort and risk

Expected change size for the recommended path:

| Area | Estimated production + test/documentation LOC |
| --- | ---: |
| Baseline fixtures and benchmark | 120-280 |
| Dependency adapter and polygon API | 250-550 |
| Blend/geometry/golden tests | 180-400 |
| Primitive migration | 150-350 |
| Host-scale diagnostics/correction | 100-300 |
| GameTerm rollout and qualification | 100-250 |
| **Total** | **900-2,130** |

The lower end applies if `tiny-skia::Mask` integrates cleanly and the host is
already presenting 1:1. The upper end applies if fixed sampling is required or
the macOS adapter needs a live backing-scale correction.

The difficult parts are not drawing one diagonal. They are:

- preserving exact composition for alpha and overlapping layers;
- retaining deterministic output across supported targets;
- avoiding full-frame temporary memory and transition-frame regressions;
- proving that the final host does not apply another accidental scale;
- changing a public default without silently invalidating consumers/goldens.

A direct Metal shape renderer would add shader/pipeline creation, tessellation,
GPU resources, synchronization, MSAA resolve targets, display-scale lifecycle,
and a separate deterministic CPU fallback. That should be treated as a future
multi-thousand-LOC renderer program, not as the answer to these four menu cards.

## Residual risks and explicit decisions

- **Browser output is not an exact golden.** The W3C permits edge AA but does
  not prescribe Safari's sample kernel. Acceptance is semantic geometry plus
  comparable perceived sharpness.
- **DDLC's 4x MSAA is corroborating evidence, not the only cause.** The menu's
  authored sprites and alpha are the more direct comparison to GameTerm's
  procedural cards.
- **`tiny-skia` determinism must be measured.** Its optimized implementations
  may differ in execution path by architecture. Exact cross-target hashes are a
  gate, not an assumption.
- **Straight versus premultiplied alpha is a halo risk.** Use the coverage mask
  with KeyGen's existing blend first; do not blindly reinterpret the entire
  surface as a premultiplied pixmap.
- **Manual density can become stale.** Fixing coverage will improve the current
  app, but moving the window across displays still needs the host audit.
- **AA changes golden pixels by design.** Update fixtures only after visual and
  mathematical review, never with an unconditional bulk regeneration.
- **Asset reuse remains the DDLC rule.** This engine improvement does not
  authorize redrawing recovered components that already exist as source assets.

## Final recommendation

Proceed with KGR-001 through KGR-004 as one bounded engine slice, then qualify
GameTerm before migrating every other primitive. Use a bbox-local
`tiny-skia::Mask` if its decision spike passes; retain a fixed 4x4 coverage
implementation as the documented fallback. Keep minifb as the presenter during
this work.

That sequence addresses the actual defect at the correct layer, gives KeyGen a
reusable 2D shape foundation, and preserves the clean-room, headless,
cross-platform architecture. It also explains the DDLC comparison cleanly:
DDLC does not ask a one-sample CPU polygon routine to create those menu edges in
the first place.
