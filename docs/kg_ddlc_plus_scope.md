# kg_ddlc_plus exhaustive macOS compatibility scope

Status: canonical product scope
Target: Apple Silicon macOS only
Source baseline: player-owned Steam app 1388880, build 10766092, bundle
version 0.1.3883356, Unity 2019.4.20f1
Output identity: kg_ddlc_plus.app

## 1. Outcome

KeyGen must compile the supported player-owned DDLC Plus installation or its
validated private recovery into one independent native application named
kg_ddlc_plus.app. The application must reproduce the supported game's boot,
MES shell, DDLC visual-novel runtime, content behavior, presentation, audio,
input, and persistence without launching or linking Unity, Ren'Py, Python,
JavaScript, a browser, a WebView, localhost, or the original executable.

The compiler translates data and behavior into KeyGen-owned versioned schemas.
It does not recompile recovered C#, embed Unity runtime libraries, claim
arbitrary Unity compatibility, or make proprietary content part of the KeyGen
repository.

~~~text
player-owned Steam application / validated private recovery
                         |
                         v
          kg_ddlc_plus source fingerprint + importer
                         |
           +-------------+--------------+
           |                            |
           v                            v
  recovered assets reused       metadata/IR translated
           |                            |
           +-------------+--------------+
                         v
           content-addressed KeyGen package
                         |
                         v
              native Rust/Metal runtime
                         |
                         v
                 kg_ddlc_plus.app
~~~

## 2. Frozen product boundaries

### Included

- Apple Silicon arm64 macOS execution and packaging.
- One canonical application identity and one canonical installed copy.
- The exact supported Steam build and an explicit rejection path for any other
  build.
- First-run and returning-user boot routing.
- BIOS, boot log, login, MES desktop, taskbar, start menu, dialogs, and all
  recovered launcher applications.
- The DDLC main menu and native visual-novel screens.
- All 34 observed story instruction descriptor variants.
- All story-specific registered capabilities needed by reachable supported
  content.
- Recovered images, sprite assemblies, fonts, audio, animations, localized
  variants, serialized layout, and material parameters.
- Keyboard, mouse, and controller input on macOS.
- Windowed/fullscreen presentation, Retina scaling, aspect policy, and black
  bars.
- Local saves, persistence, seen-text/history, unlocks, preferences, launcher
  state, virtual files, screenshots, and interruption recovery.
- Deterministic headless rendering and replay tests for non-platform logic.
- Local compilation and packaging from a legally obtained installation.

### Excluded

- Intel x86_64 and universal binaries.
- Linux, Windows, mobile, and console product builds.
- Unity Editor, Unity Player, Unity services, or Unity binary compatibility.
- Arbitrary Unity projects or arbitrary Ren'Py games.
- Steam cloud, Steam overlay, and publication under the original product
  identity. Local unlock/achievement state remains included.
- Network services, analytics, multiplayer, AI, terminal, and GameTerm product
  behavior.
- Editing or overwriting the original installation or its save directory.
- Distribution of recovered assets through Git or public build artifacts.
- Reusing recovered/decompiled C# implementation. Behavior is implemented
  independently from observed contracts and data.

General KeyGen CI may continue checking other operating systems; those checks
do not expand the kg_ddlc_plus product scope.

## 3. Asset-reuse-first contract

Asset reuse is a release requirement, not an optimization.

### 3.1 Required reuse classes

| Source class | Required treatment | Parity proof |
| --- | --- | --- |
| Texture or flat image | preserve bytes when possible; otherwise preserve decoded RGBA pixels | dimensions, pixel hash, alpha bounds |
| Sprite or atlas region | reuse pixels plus rect, pivot, border, PPU, mesh, and ordering metadata | crop hash and placement fixture |
| Character assembly | reuse body, face, and accessory sprites plus recovered child hierarchy | named-expression reference captures |
| Font | reuse recovered font bytes plus mapped TMP metrics and fallback data | glyph metrics and line-wrap fixtures |
| Audio clip | reuse decoded samples or losslessly packaged source | sample rate, channels, duration, PCM hash |
| Animation | translate recovered clip keys, tangents, wrap mode, and timing | sampled transform and opacity trace |
| Localization | reuse strings, localized sprites, fonts, and mappings | per-language layout captures |
| Layout or prefab metadata | translate hierarchy, anchors, pivots, dimensions, colors, masks, sorting, navigation, and defaults | serialized-value audit and screenshots |
| Material parameters | translate properties and texture bindings | fixed-time visual fixtures |
| Story IR | translate descriptors, labels, character/style/audio tables, and dependencies | instruction and reachability coverage |

### 3.2 Three permitted implementation modes

1. **Copy:** preserve a recovered asset payload in the private compiled package.
2. **Translate:** convert Unity-specific metadata or typed content into a
   KeyGen-owned schema without inventing values.
3. **Reimplement behavior:** independently implement engine behavior that
   cannot be reused as data, such as scene traversal, shaders, transitions,
   audio scheduling, input, save I/O, and story execution.

Every package artifact records its source logical ID, source hash, import mode,
output hash, and importer version. Absolute paths are forbidden in manifests.
Content-addressed output deduplicates repeated payloads.

### 3.3 Forbidden substitutions

- AI-generated or hand-redrawn replacements for available recovered art.
- System or approximate fonts when the required recovered font exists.
- Stock sound effects or resynthesized music when the source clip exists.
- Eyeballed pivots, anchors, positions, colors, timing, or easing when the
  serialized value is recoverable.
- Flattening a layered character into a screenshot to avoid implementing the
  recovered assembly.
- Shipping every recovered file just in case. Only reachable, declared
  dependencies may enter the local package.

Synthetic placeholder assets are permitted only in public automated tests and
must be clearly original. The macOS application icon and product identity must
be KeyGen-original so the application is not mistaken for the official game;
this exception does not permit replacement of in-game assets.

## 4. Evidence baseline and closure rule

The private recovery currently establishes:

- four serialized scenes: LauncherPreload, LauncherScene, DDLCMain, and
  SaveStateLoadingScene;
- 53 prefabs, 716 serialized assets, 82 materials, 65 animation clips, 44
  recovered or dummy shaders, two animator controllers, fonts, audio, and
  timelines;
- 137 asset bundles, 4,112 indexed assets, 2,115 sprites, 1,911 textures, and
  86 audio clips;
- a typed story object with 2,160 blocks, 287 main labels, 27,968 instruction
  descriptors, and 34 descriptor variants;
- separate launcher, game, preference, persistent, text/history, lifecycle,
  virtual-file, and unlock domains.

These counts are source-identity gates, not files to commit. A subsystem is not
complete merely because code exists: its recovered objects must be classified
as imported, unreachable with evidence, intentionally excluded by this scope,
or blocked by a named unsupported source form. The final coverage checker must
report zero unclassified reachable objects.

## 5. Target architecture

### 5.1 Repository and runtime ownership

| Owner | Responsibility | Forbidden responsibility |
| --- | --- | --- |
| keygen-engine | deterministic scene graph, animation, text layout, transitions, story VM, state-to-frame/effect reduction | filesystem, window creation, proprietary mappings |
| keygen-player | package loading, macOS host, input/audio adapters, frame presentation, lifecycle | DDLC product rules |
| kg-ddlc-plus | source fingerprinting, Unity-recovery mapping, product state machines, capability handlers, local package/app build | generic engine policy, tracked content |
| packaging tools | deterministic manifest, blob catalog, app assembly, install audit | broad deletion or original-app mutation |
| private evidence | source recovery, captures, generated manifests, conformance fixtures | Git tracking or publication |

### 5.2 Runtime state domains

~~~text
AppHost
├── PreloadState
├── LauncherState
│   ├── BIOS / BootUp / Login / Desktop
│   ├── window and app router
│   ├── files / mail / gallery / music / settings / side stories / VM
│   └── launcher progression and lifecycle
└── StoryState
    ├── execution context / variables / call and local stacks
    ├── retained stage / screens / transitions
    ├── dialogue / choice / input interaction
    ├── audio channels / active resources
    └── saves / persistent variables / seen text / unlock events
~~~

The launcher remains alive while StoryState is active. Entering or leaving the
story uses an explicit immutable handoff and deterministic teardown. Product
controllers emit typed actions; only the central router changes application
state.

### 5.3 Renderer

- Retained scene graph with stable semantic IDs and named layers.
- Logical coordinate space, recovered camera/canvas transforms, and explicit
  physical-pixel conversion for Retina displays.
- Ordered sprites, child assemblies, masks, nine-slice panels, text, canvases,
  and post-process passes.
- Metal-backed native presentation through a Rust GPU abstraction.
- Deterministic software/headless path for supported golden-test primitives.
- Before/after frame capture for dissolve, wipe, fade, and overlay transitions.
- No browser layout engine or CSS emulation.

### 5.4 Story VM

The VM is typed and fail-closed. It contains no Python interpreter and no
reflection-based arbitrary call path. Simple expressions execute in a bounded
expression evaluator. Complex game behavior is an enumerated registry of typed
native capabilities with validated arguments and explicit save policy. Unknown
descriptors, expressions, capability IDs, schema versions, or asset references
stop compilation or execution with structured diagnostics.

### 5.5 Storage

kg_ddlc_plus owns a separate application-support directory and never writes
inside the official game's installation or save tree. Stores are versioned,
checksummed, written through atomic replacement, and migrated explicitly.
Lifecycle snapshots are one-shot and deleted only after successful restore.

## 6. Functional traceability matrix

### 6.1 Bootstrap and launcher

| Reference surface | Required KeyGen behavior | Reused data and assets | Packets |
| --- | --- | --- | --- |
| Launcher preload | initialize paths, preferences, input, audio, localization, stores, and catalog | preload settings and serialized defaults | KGD-110, 120, 520 |
| First-run branch | route using interaction marker and story return state | recovered defaults and flags | KGD-201, 202 |
| BIOS | timed lines, logo, buzz, skip and completion route | text, font, logo, audio, layout | KGD-210 |
| BootUp | timed log, spinner, aspect-dependent line count, skip, audio | text, font, sprites, audio, layout | KGD-211 |
| Login | input gate, store load, transition to desktop | hierarchy, animation, text, audio | KGD-212 |
| Central router | start, update, close, input lock, focus restore, persistence | app registry and defaults | KGD-201 |
| Desktop | wallpaper, clock, icons, taskbar, start menu, notifications | sprites, prefabs, fonts, animations | KGD-220 |
| Shared windows | scale, opacity, panel, and taskbar choreography | clips, curves, layout | KGD-221 |
| Confirmation and errors | modal ownership, focus restore, quit and save guards | panels, icons, strings, navigation | KGD-222 |
| File Browser | hierarchy, selection, open, delete, and reset rules | entries, icons, strings | KGD-230 |
| File Viewer | text, sprite, and audio modes | viewed assets and metadata | KGD-231 |
| Mail | browse, read, timestamps, and notifications | content, metadata, layout | KGD-232 |
| Side Stories | unlock projection and story launch handoff | entries, thumbnails, layout | KGD-233 |
| Gallery | categories, unlocks, and full-image presentation | metadata and images | KGD-234 |
| Jukebox | browsing, playback, and mini-player lifecycle | audio, covers, metadata | KGD-235 |
| Settings | text and auto speeds, volume, display, language, VSync, commentary, warnings | controls, strings, defaults | KGD-236 |
| VM | deterministic terminal, glitch sequence, and reset route | text, font, audio, effects, timing | KGD-237 |
| DDLC handoff | enter story, suspend launcher presentation and audio, restore on return | launch parameters and dependencies | KGD-240 |

### 6.2 Visual-novel presentation and screens

| Reference surface | Required KeyGen behavior | Reused data and assets | Packets |
| --- | --- | --- | --- |
| Camera and canvas policy | reference spaces, scaling, crop or bars, sorting | cameras and canvases | KGD-310, 510 |
| Retained image layers | show, hide, scene, named layers, z-order, child assembly | textures, sprites, prefab hierarchy | KGD-311, 312 |
| Transforms and ATL | immediate, ease, time, parallel, repeat, and event behavior | descriptors and animations | KGD-313 |
| Transitions | before and after capture, fade, dissolve, wipe, overlay | materials, masks, defaults | KGD-314 |
| Dialogue | name and text panels, styles, speaker focus, CTC, window states | panels, fonts, styles, focus maps | KGD-320 |
| Typewriter | reveal, punctuation waits, skip, auto-forward, dismiss timing | strings, preferences, measured cadence | KGD-321 |
| Choice | conditional entries, focus, navigation, result handoff | panels, fonts, navigation | KGD-322 |
| Main menu and navigation | title menu, actions, and state | UI sprites, layout, fonts, audio | KGD-323 |
| History | encountered dialogue and scrolling | layout and styles | KGD-324 |
| Save and load | 54 slots, screenshots, paging, confirmation, restore | screen assets and layout | KGD-325, 430 |
| Preferences | story-facing preferences and live projection | controls, layout, strings | KGD-326 |
| Name input | validated entry and macOS text input | screen assets and layout | KGD-327 |
| Poem and poetry game | word selection, affinities, instructions, poem display | words, sprites, audio, UI, locales | KGD-328 |
| Special screens and effects | quick menu, tear, invert, disconnect and account fallback | screens and materials | KGD-329 |

### 6.3 Story descriptor coverage

All observed descriptor variants are mandatory. Implementation packets may
share a VM core, but the coverage checker treats each variant independently.

| Descriptor group | Variants | Required semantics | Packets |
| --- | --- | --- | --- |
| Dialogue and UI | Dialog, Window, WindowAuto, Text, WaitForScreen, MenuInput | interaction boundaries, screen results, history and seen state | KGD-320–322, 407 |
| Images and scenes | Show, Hide, Scene, LoadImage, Size | retained mutations, dependency checks, queued changes | KGD-311, 404 |
| Transitions and transforms | With, Immediate, Ease, Time, Pause | clocked transforms and transition boundaries | KGD-313, 314, 405 |
| Audio | Play, Stop, Queue | music, poem, and sound channel state, fade, loop, queue | KGD-330, 406 |
| Control flow | LabelEntryPoint, Goto, Return, GotoLine, GotoLineUnless, GotoLineTimeout, ForkGotoLine | cursor, stacks, conditions, time and input forks | KGD-401–403 |
| Native behavior | Function, OneLinePython, InlinePython | bounded expression or registered typed capability only | KGD-410–413 |
| Data and flags | Expression, SetRandRange, Unlock, ClrFlag, NOP | deterministic mutation, seeded randomness, progression events | KGD-408, 414 |

### 6.4 Persistence, files, localization, input, and audio

| Domain | Required behavior | Required stores or adapters | Packets |
| --- | --- | --- | --- |
| Runtime saves | cursor, stacks, scopes, images, transforms, screens, history, channels, screenshot | 54 slots plus autosave and lifecycle | KGD-430, 431 |
| Persistent story | cross-run variables and progression | independent atomic store | KGD-432 |
| Seen text and history | skip/read state and history | independent store | KGD-433 |
| Launcher | apps, per-app state, wallpaper, unlock projection, jukebox | launcher and one-shot lifecycle stores | KGD-434 |
| Preferences | speeds, volumes, display, language, skip, mute, warnings, VSync | shared preference store | KGD-435 |
| Interaction marker | first-run routing | minimal independent marker | KGD-202 |
| Virtual files | catalog, visibility, timestamps, deletion policy, stale reconciliation | sandboxed application-support files | KGD-440 |
| Unlocks | local events and progress consumed by shell and story | progression service and null external provider | KGD-441 |
| Localization | lines, UI strings, sprites, fonts, dependencies, live refresh | locale catalog and asset variants | KGD-150, 442 |
| Input | mouse, keyboard, controller modality, focus graph, locks, glyphs | macOS adapters and deterministic actions | KGD-500–502 |
| Audio | music, poem, sound, launcher and jukebox ownership; queue, fade, restore | macOS adapter and channel state | KGD-330–332, 503 |
| Lifecycle | quit interception, save completion, activate, deactivate, interruption restore | macOS lifecycle adapter | KGD-520, 521 |

## 7. Import and package contracts

### 7.1 Supported input path

The compiler must ultimately accept the owned Steam app as its operator input.
It fingerprints the application and build, locates or produces a private
recovery cache using an approved pinned extraction path, then imports from that
cache. The current direct ExportedProject input remains a supported diagnostic
mode. The extraction tool and its redistribution and invocation license must be
verified before it is automated or bundled.

### 7.2 Required versioned documents

- keygen.package: identity, entry state, blob catalog, dependencies,
  provenance, compiler version, and schema compatibility.
- keygen.scene.v2: retained nodes, cameras, canvases, masks, text, materials,
  animations, focus semantics, and stable IDs.
- keygen.story.v1: blocks, labels, descriptor variants, expressions,
  capability calls, character, style, and audio tables, and source locations.
- keygen.assets.v1: logical IDs, locale and label variants, content hashes,
  dependencies, source metadata, and import mode.
- keygen.launcher.v1: app registry, initial routing, window defaults, and
  content mappings.
- Separate save schemas for runtime, persistent, seen text, launcher,
  preferences, and lifecycle state.

Every schema is versioned, denies unknown fields, bounds sizes and counts, uses
relative logical paths, and emits a precise diagnostic for unsupported input.

### 7.3 Reachability and packaging

The compiler begins from preload, launcher, and story entry roots and resolves
scene, prefab, label, localization, and asset dependencies. It packages the
reachable closure only. Label-scoped groups remain separately loadable so the
runtime can release story resources without destroying the launcher.

Package validation rejects missing blobs, hash mismatches, undeclared files,
absolute paths, traversal, unknown schemas, unclassified reachable descriptors
or capabilities, and substituted source assets when recovery data was
available.

## 8. macOS application contract

The canonical bundle is:

~~~text
kg_ddlc_plus.app/
└── Contents/
    ├── Info.plist
    ├── MacOS/kg_ddlc_plus
    ├── Resources/keygen/package.json
    ├── Resources/keygen/blobs/...
    └── Resources/<KeyGen-owned product icon and metadata>
~~~

- Architecture: arm64 only.
- Renderer: Metal-backed native window; no embedded browser.
- Product and save identity are distinct from the official game.
- Build output: one deterministic bundle under ignored dist.
- Install target: exactly /Applications/kg_ddlc_plus.app after validating the
  explicit target. Install logic never scan-deletes broad or similar paths.
- Spotlight and Finder expose the canonical product name.
- Rebuilding replaces only the exact target after successful staged validation.
- Development and recovery caches stay outside the installed bundle unless a
  file is in the declared reachable package.
- Ad-hoc signing is sufficient for the owner's local build. Distribution
  signing, notarization, and public release are outside current scope.
- KGD-101 freezes the minimum macOS version after dependency qualification.

## 9. Quality attributes and budgets

| Attribute | Required gate |
| --- | --- |
| Determinism | same source, compiler, and options produce identical manifests, story IR, and headless frames |
| Fidelity | exact provenance plus visual, audio, timing, and behavior comparisons |
| Safety | no write to official paths; relative package paths; bounded decoding |
| Diagnostics | stable error codes and logical IDs; no private absolute paths in reports |
| Startup | no network, editor, account, or service dependency; measured budget |
| Frame pacing | sustained recovered target cadence on the qualification Mac |
| Memory | label-scoped release and leak-free launcher and story round trips |
| Recovery | interrupted writes preserve last valid save; one-shot lifecycle restore |
| Accessibility | semantic focus order, keyboard operation, readable recovery UI |
| Repository hygiene | hooks pass, proprietary files ignored, source under sz.py cap |

Pixel comparison alone is insufficient: input locks, focus ownership, timing,
audio ownership, save state, and route transitions are observable contracts.

## 10. Qualification matrix

Private reference evidence is captured at fixed build, resolution, locale,
state, and timestamp. At minimum, qualification covers:

- fresh install, first interaction, returning launch, and lifecycle restore;
- BIOS start, midpoint, end, skip; BootUp start, spinner, end, and skip;
- login idle and activate, then desktop arrival;
- every desktop app open, primary state, modal path, close, and focus restore;
- title menu idle, every navigation entry, settings, confirmation, and return;
- short, long, formatted, and localized dialogue; auto, skip, history, speaker
  focus, and click-to-continue;
- each character's reachable pose and expression assembly and representative
  overlap or layer cases;
- each transition and material family at start, midpoint, and end;
- choices, pauses, timeouts, branches, calls, returns, forks, and every
  descriptor variant;
- music and sound queue, fade, restore, and launcher, story, jukebox ownership;
- save, load, autosave, lifecycle, and corrupt-save recovery;
- virtual-file creation, visibility, deletion, and reset flows;
- locale changes with font and sprite reload plus layout reflow;
- windowed and fullscreen, Retina and logical scaling, 16:9 and the owner's
  MacBook aspect ratio;
- controller connection, disconnection, and modality glyph changes;
- repeated desktop-to-story cycles and quit during an active save.

Each case records expected state, actions, timestamps, semantic output,
reference capture IDs, asset logical IDs, and allowed tolerance. Tolerances are
defined per primitive after measurement; agents may not invent a global
looks-close threshold.

## 11. Completion gates

| Gate | Exit condition |
| --- | --- |
| G0 Source | exact fingerprint, complete aggregate inventory, no tracked private content |
| G1 Assets | every reachable asset classified, reuse manifest complete, zero forbidden substitutions |
| G2 Core | schemas stable; deterministic renderer and VM; structured diagnostics |
| G3 Launcher | first-run and returning boot through every launcher app passes fixtures |
| G4 VN shell | title, navigation, dialogue, choice, screens, transitions, audio, and input pass |
| G5 Story | all reachable blocks, labels, descriptors, and capabilities compile and replay |
| G6 State | stores, saves, files, unlocks, preferences, localization, and recovery pass |
| G7 Application | one arm64 kg_ddlc_plus.app builds, validates, installs, launches, quits, and is searchable |
| G8 Qualification | matrix complete, no critical parity issue, budgets pass, repeated clean run passes |

The end goal is reached only when G0 through G8 are green. Boots, shows the
menu, and most story commands work are intermediate milestones.

## 12. Delegation rule

Implementation is governed by
[kg_ddlc_plus_tasks.md](kg_ddlc_plus_tasks.md). Only packets whose dependencies
are complete may run concurrently. Agents own only their packet paths, preserve
private evidence, and return the named acceptance evidence. If recovered data
contradicts this scope, the evidence or schema packet updates the contract
before implementation; feature agents do not guess.
