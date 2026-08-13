# Two usable menus: first-principles learnings

Date: 2026-08-12

## Scope

This retrospective compares the two recovered, interactive menu surfaces built
on the same clean-room Rust substrate:

1. **DDLC title menu** — the inner 1280×720 visual-novel menu with the moving
   background, four character layers, logo, navigation plate, particles,
   entrances, fade, and four selectable text actions. This began in the
   GameTerm Beta repository at commit `fc5cd9a` and became the first KeyGen
   compositor proof.
2. **DDLC Plus MES launcher menu** — the outer 1920×1080 desktop/start menu with
   wallpaper, panel, shadow, taskbar, clock, eight focusable rows, icon variants,
   package routes, Back behavior, and borderless macOS presentation. The
   completed checkpoint is KeyGen commit `5dc9db3`.

Both are usable menu screens: they render from local recovered assets, accept
keyboard and pointer input, maintain focus, and emit or route an activated
semantic action. They do not imply that every downstream game application or
story is complete.

## Executive conclusion

The first menu proved that we could reproduce a bounded Unity-authored 2D
surface without Unity. The second menu tested whether we had built a reusable
engine or merely a renderer shaped like the first screenshot.

The most important result is this separation:

```text
player-owned recovered assets and metadata
                    |
                    v
       product-specific scene compiler
                    |
                    v
     title-neutral KeyGen scene contract
                    |
          +---------+---------+
          |                   |
          v                   v
 deterministic renderer   native macOS host
 pixels + hit geometry    window + input + time
```

A faithful menu is not a screenshot and not merely a list of buttons. It is:

```text
assets
  + layout and coordinate semantics
  + ordered composition
  + focus and input state
  + animation and timing
  + action routing
  + native host presentation
  = the menu the player experiences
```

If we did this again, we would audit the source into an explicit menu contract
before drawing anything, implement only missing reusable primitives, and test
the installed application from the beginning. We would use the first screen as
the vertical slice and the second as the generality test.

## What each menu taught us

| Concern | DDLC title menu | MES launcher menu | Combined lesson |
| --- | --- | --- | --- |
| Composition | Large layered art, characters, logo, navigation plate | Desktop chrome, panel, shadow, atlas row, icons, taskbar | A menu is an ordered scene graph even when it appears visually flat. |
| Motion | Background drift, character entrances, logo bounce, particles, white fade | Mostly stable desktop with state-driven focus layers | Animation and discrete UI state belong in the same deterministic frame model. |
| Selection | Outlined text changes with one focused index | Selected-row texture, highlighted icon, focused text, full-row hit area | Focus is one semantic state projected into several visual and behavioral outputs. |
| Assets | Mostly standalone PNG layers plus font | Standalone sprites, sprite-atlas crops, and nine-sliced UI | An importer must preserve asset semantics, not just copy image files. |
| Geometry | Reconstructed 1280×720 placement and insertion order | Exact 1920×1080 prefab geometry and Unity-to-PNG crop conversion | Establish one canonical design space and translate every source coordinate into it. |
| Typography | One external display font with outline | Multiple font roles, live clock, and a missing source TTF for one TMP SDF role | Typography is a subsystem; a font filename alone does not describe its output. |
| Behavior | Up/Down, hover, click, Enter/Space, Escape, action IDs | The same focus loop plus package route loading and Back restoration | Rendering, hit testing, and routing must share semantic IDs. |
| Host | Resizable native proof window | Display-fit borderless macOS app with aspect preservation and system-UI policy | Window geometry and OS chrome are part of visual fidelity. |
| Validation | Headless/native use the same compositor | Private package build, hashes, installed app smoke, route/back checks | The installed artifact—not a development PNG—is the final test subject. |

## The biggest architectural lesson: screen two is the real abstraction test

It is easy to create an API that looks generic while implementing the first
screen. The first title menu needed ordered images, simple anchors, uniform
scales, motion, entrance easing, particles, text, and focus. Those concepts
could have been accidentally tailored to its exact composition.

The launcher immediately asked different questions:

- Can one image layer show only a source rectangle from an atlas?
- Can a shadow stretch without deforming its corners?
- Can a layer be visible only for a particular focused row?
- Does hiding that layer preserve the insertion/z-order of later content?
- Can focused text change fill rather than only outline?
- Can one text layer use a different font from the menu?
- Can a host-provided clock update without making the compositor nondeterministic?
- Can the package request borderless display-fit presentation without embedding
  DDLC rules in the player?
- Can activation load a declared scene and can Back restore the launcher?

Those gaps produced reusable features—atlas crops, nine-slice, focus-conditioned
layers, focused colors, per-text fonts, host-injected clock values, declarative
window policy, and scene routing. None of those primitives contains a DDLC
name, asset, coordinate, or route.

This gives us a practical rule:

> Do not call an engine abstraction reusable after one screen. Build a second
> screen with a different composition grammar and observe what breaks.

## 1. Start with evidence, not an approximation

The correct evidence hierarchy is:

1. behavior observed in the installed application;
2. serialized scene and prefab hierarchy;
3. sprite, atlas, font, material, animation, and audio metadata;
4. player-owned recovered asset bytes;
5. names and code-shaped metadata as clues;
6. visual estimation only where stronger evidence is absent.

Each source answers a different question:

- A screenshot shows what appeared in one state at one moment.
- A prefab shows object relationships, active states, anchors, pivots, sizes,
  references, and defaults.
- Sprite metadata shows that an apparently standalone visual may really be one
  rectangle inside an atlas.
- Runtime observation reveals selection, timing, transitions, action results,
  Back behavior, and native window treatment.

The first title menu was reconstructed from local assets, recovered transforms,
timing, and a screenshot oracle. The second pass was stronger because the
launcher audits recorded exact panel dimensions, row cadence, atlas rectangle,
icon variants, shadow, and canvas size before implementation. The fewer values
we guess, the fewer rounds of visual correction we need.

For future menus, the first deliverable should be a contract containing:

- canonical design resolution;
- entry and exit state;
- every visible node in z-order;
- source asset and source rectangle;
- anchors, pivots, position, destination size, and scale;
- nine-slice borders, masks, alpha, and material treatment;
- font role, metrics, alignment, fill, outline, and shadow;
- normal, focused, hovered, pressed, and disabled variants;
- complete hit regions and navigation order;
- entrance, idle, selection, and exit timing;
- semantic action IDs and destination states;
- native window, aspect, and fullscreen behavior;
- evidence location and confidence for every non-obvious value.

The [launcher source audit](ddlc-launcher-boot-audit.md) and
[comparison audit](ddlc-launcher-comparison-audit.md) are useful examples of
separating source facts from candidate-renderer gaps.

## 2. Preserve semantics, not Unity implementation

We did not need Unity's editor or player to draw either menu. We did need the
observable meaning of the source:

- ordered layers become ordered compositor nodes;
- `RectTransform` values become design-space geometry;
- sprite rectangles become source crops;
- sliced sprites become nine-slice drawing;
- active/focused variants become conditional layers;
- animation curves become named, deterministic easing;
- Unity UI navigation becomes semantic focus movement;
- button callbacks become typed route/action IDs;
- canvas/window policy becomes package data plus a host adapter.

This is a translation, not a reimplementation of Unity's internal architecture.
The reusable engine should not know which game supplied the data. The private
product compiler is allowed to know that a particular recovered sprite and
rectangle implement a particular launcher row.

The clean boundary is:

- **KeyGen engine:** validated data in, deterministic pixels/state out.
- **KeyGen player:** bounded file loading, window, input, clock, and lifecycle.
- **Compatibility compiler:** game-specific mapping from audited source facts
  to KeyGen documents.
- **Private package:** player-owned asset bytes and generated scenes, ignored by
  Git.

## 3. Reuse-first is the fastest route to fidelity

Most early visual differences were not failures of rasterization. They were
failures to use an existing source component correctly.

Examples from the two menus:

- The title menu's navigation plate, logo, characters, and particle sprite were
  separate layers with explicit insertion points; flattening them would lose
  entrances and occlusion.
- The launcher panel was a textured sprite, not a flat rectangle.
- The launcher focus row was a crop from a larger atlas, not a generated pink
  fill.
- The icon set had separate normal and focused assets.
- The drop shadow needed nine-slice behavior rather than uniform scaling.
- The taskbar was another atlas crop rather than a newly drawn bar.

The rule is:

```text
If the source contains the visual, import it.
If the source contains the value, translate it.
If the source contains the relationship, model it.
Approximate only when the evidence is absent, and document the gap.
```

Reuse does not mean turning the whole menu into one screenshot. We reuse the
smallest meaningful components so they retain state, animation, layering,
scaling, and interaction.

## 4. One coordinate system must govern pixels and input

Both menus use a fixed logical design surface and scale that surface into a
native window. That decision gives us stable composition, deterministic
headless renders, and consistent pointer mapping.

The conversion must still handle several traps:

- Unity and PNG crop coordinates can use different vertical origins. For atlas
  height `H`, bottom-origin rectangle `y` and height `h` convert with:

  ```text
  y_top = H - (y + h)
  ```

- A sprite's source pixels do not necessarily equal its displayed dimensions.
- Pivot/anchor interpretation must be resolved before host scaling.
- Nine-slice centering uses final destination dimensions, not source dimensions.
- Letterbox offsets must be included when mapping host pointer coordinates back
  into design space.
- Hit boxes come from semantic row geometry, not the visible glyph bounds.

Rendering and hit testing cannot have separate geometry implementations. If the
pointer mapper, compositor, and menu contract do not share the same design
surface, the screen can look right and still feel broken.

## 5. Z-order must be data

The title menu demonstrated why “draw all backgrounds, then characters, then
UI” is not expressive enough. The navigation plate, logo, particles, Sayori,
and Monika have specific insertion relationships. The launcher added a second
subtlety: a focus-conditioned layer still owns its declared z-order slot when
it is hidden.

Therefore:

- layers stay in a single declared order;
- menu and particle insertions refer to stable positions in that order;
- conditional visibility changes whether pixels are drawn, not the topology of
  the composition;
- product compilers determine the exact order from source evidence.

This is one of the clearest cases where a screenshot-only renderer would have
hidden an important runtime behavior.

## 6. Focus is one state with many projections

For the title menu, focus changed text treatment and determined which action
Enter/Space activated. For the launcher, the same state also selected a row
texture and highlighted icon.

```text
focused semantic entry ID/index
       |          |           |           |
       v          v           v           v
  row layer   icon layer   text style   activated route
       |
       v
pointer and keyboard hit/navigation state
```

All of these must read the same focus authority. Implementing them independently
causes the highlight/action mismatch that users perceive immediately.

The reusable interaction contract now needs:

- one semantic ID per entry;
- enabled/disabled state;
- one ordered focus model for keyboard and pointer;
- a full design-space hit rectangle;
- visual projections for every state;
- one activation result consumed by a router;
- deterministic Back and Exit behavior.

## 7. Timing belongs in the scene model

The first menu made this unavoidable. Its identity depends on background drift,
staggered character entrances, navigation movement, logo bounce, particle
bursts, and fades. A settled screenshot is useful evidence, but it is not the
whole menu.

We learned to express time as deterministic inputs and declarative values:

- delay and duration;
- named easing;
- start/end transform values;
- particle start/lifetime/velocity;
- fade and flash timing;
- a supplied elapsed time for headless capture.

The same scene and same elapsed time must always produce the same pixels. The
native loop supplies time; the compositor never reads a clock. The launcher's
wall clock follows the same boundary principle: the host reads local time and
projects an ordinary string into a marked text layer.

## 8. Typography is more than choosing a similar font

The first menu needed an external display font and thick outlined labels. The
second exposed multiple roles: menu text, desktop clock text, and Unity
TextMeshPro SDF data.

The actual output depends on:

- font family and exact face;
- glyph metrics and kerning;
- size and line height;
- fill and outline;
- raster antialiasing or SDF material behavior;
- alignment and baseline;
- per-role font selection.

One scene-global font was insufficient, which led to per-text font overrides.
There is also an honest remaining limitation: the launcher recovery contains
Vera TextMeshPro SDF metadata and an atlas but no standalone Vera TTF. The
current menu uses a neutral recovered raster fallback for that role. Exact
TextMeshPro fidelity requires a bounded SDF font/material importer and renderer,
not more visual guessing.

This taught us to record typography gaps precisely instead of claiming pixel
identity when the source rendering pipeline is not yet implemented.

## 9. The native host is part of visual parity

The first menu proved a resizable native window. The launcher showed why that
was not sufficient for every product surface. The same correct 1920×1080 frame
looked different when macOS added a title bar, menu bar, Dock, or an unintended
window size.

The second menu therefore required declarative host policy:

- normal versus borderless window;
- fixed size versus fit to the active display;
- aspect preservation and black bars;
- logical display points versus backing pixels;
- system UI visibility;
- pointer mapping through scaling and letterboxing;
- correct behavior while drawable size is temporarily zero.

These are player/host responsibilities, but the scene/package must be able to
request them without a DDLC-specific branch.

Live evidence also needs environmental controls. Mission Control once made a
full-display borderless surface appear like a small window in a screenshot.
Before judging a capture, record the frontmost process, window geometry,
display geometry, and OS presentation state.

## 10. A menu is not usable until actions have destinations

The first screen gave us semantic action emission. The second forced the next
step: manifest-declared routes, scene loading, viewport validation, and Back
restoration.

A complete menu boundary is:

```text
normalized input
  -> focus reducer
  -> semantic action
  -> central router
  -> destination state/scene
  -> Back restores the prior owner and focus policy
```

The compositor must not interpret product actions. It returns a semantic ID.
The central router owns state transitions, and the package manifest declares
where compatible actions lead. Unknown, missing, unsafe, or mismatched scene
routes fail closed.

This also keeps “the menu works” distinct from “every application behind the
menu is implemented.” We can qualify the menu boundary without overstating full
game coverage.

## What we would not do again

1. **Approximate before inventorying.** Generic rectangles and guessed spacing
   create rework when exact panels, rows, and crops already exist.
2. **Treat a menu as text over a background.** Both menus are layered,
   time-aware compositions with stateful visuals.
3. **Flatten the composition.** It removes animation, focus variants,
   occlusion, responsive scaling, and interaction.
4. **Infer hit boxes from glyphs.** Visual text bounds are not semantic row
   bounds.
5. **Use cadence as geometry.** A 52px row plus 21px gap repeats every 73px but
   is not equivalent to a true 73px interactive row.
6. **Let renderer defaults define the product.** Default window chrome and
   scaling materially change the result.
7. **Use one font for every role.** Menu and desktop typography can have
   different source pipelines.
8. **Validate only a settled PNG.** Entrance timing, focus, action routing,
   Back, Exit, and host presentation require their own evidence.
9. **Claim exactness without exact evidence.** Missing SDF support and inferred
   transforms must be named as bounded gaps.
10. **Put compatibility data in engine code.** Coordinates, local asset paths,
    product labels, and routes belong in the private projection/package.
11. **Test only the development executable.** Users run the installed `.app`
    and its packaged resource graph.
12. **Assume screen one proves generality.** Screen two is where hidden
    coupling becomes visible.

## The workflow we would use next time

### Phase 0: define one menu boundary

Name the exact surface, predecessor, successor, supported source build, target
viewport, target host, and completion claim. Separate an outer launcher menu
from the inner game title menu even if a user informally calls both “boot.”

### Phase 1: freeze the evidence

Fingerprint the supported recovery. Inventory the relevant scene/prefab roots,
sprites, atlas rectangles, fonts, materials, animation/timeline values, audio,
callbacks, and native observed states. Keep all private bytes outside Git.

### Phase 2: write a screen contract

Produce the layer order, geometry conversion table, focus/action table, timing
timeline, route graph, and native-host policy. Every non-obvious value gets a
source and confidence.

### Phase 3: audit engine capability gaps

Classify every need as:

- an existing generic primitive;
- a missing generic primitive;
- a product-specific mapping;
- a host/platform behavior;
- unresolved source evidence.

Implement the smallest generic primitive justified by the screen, then prove it
with synthetic assets and unit tests before using private content.

### Phase 4: compile the private scene

Resolve exact player-owned assets and serialized values into a versioned KeyGen
scene/package. Fail if required assets or expected dimensions do not match.
Never silently select the first vaguely similar image or font.

### Phase 5: connect interaction and routing

Use common semantic IDs for visual states, hit testing, keyboard focus,
activation, routes, Back, and Exit. Keep the reducer deterministic and host
effects explicit.

### Phase 6: qualify in concentric loops

```text
schema and asset validation
  -> primitive unit tests
  -> deterministic timed headless captures
  -> focus/action/route replay
  -> package graph and hash checks
  -> installed native app smoke
  -> controlled live visual and input comparison
```

When a mismatch appears, identify whether it comes from evidence, mapping,
engine primitives, or host presentation. Avoid unexplained one-off pixel nudges.

### Phase 7: use a second screen as the graduation test

The second screen should have a meaningfully different composition grammar.
Track every engine change it requires. If those changes remain title-neutral
and improve both screens, the abstraction is holding. If they add product IDs
or special cases, move the logic back to the compiler or redesign the contract.

### Phase 8: checkpoint honestly

Commit generic implementation, tests, importer logic, and aggregate evidence.
Do not commit recovered content. Record passing commands, installed artifact
identity, known gaps, and the exact next screen boundary.

## Four independent completion gates

| Gate | Question | Evidence |
| --- | --- | --- |
| Structural | Did we reconstruct the right source objects and relationships? | Source-reference, layer-order, asset, and geometry audit |
| Visual/temporal | Does the correct state at the correct time match at a controlled viewport? | Same-state reference/candidate captures at named timestamps |
| Behavioral | Do focus, pointer, keyboard, activation, routes, Back, and Exit work? | Deterministic input/action/state traces |
| Host/package | Does the actual installed application present and close correctly? | Resource hashes, no-argument launch, window geometry, smoke, no crash/leak |

No single gate substitutes for the others. A screenshot cannot prove routing.
A route unit test cannot prove atlas orientation. A successful process launch
cannot prove typography or animation fidelity.

## Reusable checklist for menu three

- [ ] The menu has an unambiguous name and state boundary.
- [ ] The exact supported source recovery is fingerprinted.
- [ ] Every visible component is traced or marked unresolved.
- [ ] One canonical design coordinate system governs render and input.
- [ ] Anchors, pivots, atlas origins, crop rectangles, scales, and nine-slice
      borders are translated and tested.
- [ ] Layer order and all insertion points are explicit.
- [ ] Normal, focused, hovered, pressed, and disabled projections share one
      semantic focus authority.
- [ ] Text roles and material limitations are documented.
- [ ] Timings and transitions are deterministic and capturable at fixed times.
- [ ] Every action has a typed semantic ID and a defined result.
- [ ] Route failure, Back, and Exit behavior are tested.
- [ ] Host window, aspect, display, and system-UI policies are intentional.
- [ ] Headless and native presentation use the same compositor.
- [ ] The installed package validates adjacent resources and hashes.
- [ ] Reference and candidate captures use the same viewport, state, and time.
- [ ] No proprietary assets, recovered source, or machine-specific paths are
      tracked.
- [ ] Formatting, lint, unit tests, package checks, private-content scan, and
      clean Git status pass.
- [ ] Remaining differences are named rather than hidden behind a parity claim.

## Bottom line

Two usable menus taught us more than twice what one menu did.

The DDLC title menu established the minimal deterministic 2D compositor and
input loop. The MES launcher then forced that substrate to handle a different
visual language, richer asset semantics, more exact focus projection, routing,
and native macOS presentation. That second pass is what turned the work from a
convincing demo into the beginning of a reusable engine architecture.

We still have not reproduced all of Unity, and we do not need to in order to
continue. For bounded 2D game surfaces, we now have a repeatable method: recover
facts, compile semantics into title-neutral data, render deterministically,
route semantic actions, and qualify the installed artifact.
