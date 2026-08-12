# kg_ddlc_plus execution ledger

This ledger decomposes the
[canonical scope](kg_ddlc_plus_scope.md) into bounded, dependency-ordered work
packets suitable for low-effort implementation agents. A checked box means its
acceptance commands and evidence exist, not merely that code was started.

## Packet contract

Every assignment must include:

- one packet ID and no unrelated cleanup;
- its prerequisite commit IDs;
- exact writable paths, narrowed from the path family below before launch;
- read-only source/evidence paths;
- deliverables and acceptance commands from this ledger;
- a prohibition on tracked proprietary content, private absolute paths,
  recovered C#, or generated substitutes for available assets;
- a requirement to leave a short report under
  docs/task_reports/<packet-id>.md containing changed paths, tests, source
  counts, limitations, and follow-up risks.

Agents must stop with an explicit unsupported diagnostic when source evidence
is insufficient. They do not invent layout, timing, asset, or behavior values.
Only packets in the same declared parallel wave may run simultaneously.

Shared integration paths such as workspace Cargo.toml files, crate lib.rs
module lists, schemas, and package manifests belong to the wave's integration
packet. Feature packets work in dedicated module and test paths until
integration. The primary agent resolves integration, runs full gates, commits,
and pushes main after every wave.

## Status and waves

| Wave | Packets | Parallel rule | Exit |
| --- | --- | --- | --- |
| 0 current slice | KGD-000 | complete | reconstructed BIOS package and native smoke |
| 1 governance and evidence | 001–006, 101–104 | 001 first; 002–006 parallel; 101–104 after evidence schema | source and policy gates |
| 2 asset compiler | 110–115, 120–132, 140–142, 150 | importers parallel by asset class after 120; 132 and 142 integrate | G0 and G1 |
| 3 general runtime and state foundations | 301–318, 330–332, 400–411, 430, 435, 440, 441, 500 | parallel only by dedicated module; integration packets run last | G2 foundations |
| 4 launcher | 201–240, 434 | router first; apps parallel after shared windows; 240 integrates | G3 |
| 5 VN shell and interpreter | 320–329, 412–420 | inventories before generated handlers; screens parallel after presentation core | G4 |
| 6 content and remaining state | 421–425, 431–445 | capability batches and independent stores parallel after their foundations | G5 and G6 |
| 7 macOS application | 500–533 | adapters parallel; bundle after runtime adapter integration | G7 |
| 8 qualification | 600–611 | capture/diff tools parallel; full qualification serial | G8 |

## Wave 0: implemented vertical slice

### KGD-000 — source-to-BIOS proof

Status: complete in commit 8e4f926.

Owns: crates/kg_ddlc_plus, timed text support in engine/player, local package
manifest, operator documentation, hooks, and sz.py.

Acceptance already recorded: exact build fingerprint, local-only copied font
and logo, parsed BIOS timing, package validation, deterministic 1920×1080 PNG,
one-second native smoke, full workspace format/clippy/tests, and ignored local
output.

This is scaffolding. Later schema/runtime packets may replace its simple scene
format while retaining a migration test.

## Wave 1: governance and evidence

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-001 | Machine-readable ledger and checker under tasks/ and scripts/check-kg-scope.* | 000 | validates unique IDs, dependencies, legal status transitions, report links, and no cycle |
| KGD-002 | Private-content and absolute-path scanner under scripts/check-private-content.* | 001 | seeded proprietary-name/hash/path fixtures fail; repository and staged diff pass |
| KGD-003 | Structured diagnostic code catalog under crates/diagnostics/ | 001 | stable code, severity, logical source ID, redacted rendering, JSON round trip |
| KGD-004 | Private evidence manifest schema under docs/evidence/ and crates/kg_ddlc_plus/src/evidence/ | 001,003 | records build, state, locale, resolution, timestamp, capture hashes, no asset bytes or absolute paths |
| KGD-005 | Reproducible observation protocol and operator scripts under scripts/private/ | 001,004 | read-only dry run, hash log, state checklist, no original-install writes |
| KGD-006 | License/provenance decision record for extraction and local packaging under docs/decisions/ | 001 | identifies what may be invoked, bundled, copied locally, tracked, and distributed; unresolved terms block automation |
| KGD-101 | arm64 macOS target and deployment-floor decision under docs/decisions/ and rust config | 002 | arm64 binary proof, dependency support table, Intel/universal product jobs absent |
| KGD-102 | Canonical product, bundle, save, cache, and install identities | 002,101 | none collide with official DDLC identifiers or GameTerm; unit tests lock values |
| KGD-103 | KeyGen module-boundary ADR and dependency rules | 001 | engine cannot depend on player/product/importer; automated dependency check passes |
| KGD-104 | Performance/size metric harness with unset thresholds | 001,101 | measures startup, frame pacing, resident memory, package size; does not invent budgets |

Wave 1 exits when the ledger, privacy scan, diagnostic schema, evidence schema,
identity rules, and arm64 platform decision all run in fast/full hooks and CI
where public fixtures permit.

## Wave 2: recovery, asset reuse, and package compiler

### Source and inventory

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-110 | Official Steam app discovery and read-only fingerprint adapter under kg_ddlc_plus/source/ | 002,003,006,102 | exact build accepted; missing, changed, and wrong-architecture apps fail with stable diagnostics |
| KGD-111 | ExportedProject diagnostic-source adapter | 002,003 | rejects incomplete trees; records logical paths and hashes without absolute paths |
| KGD-112 | Pinned extraction-cache orchestration, only if KGD-006 permits | 005,006,110 | dry run is read-only; cache is ignored; tool/version/hash recorded; interruption leaves original untouched |
| KGD-113 | Aggregate source inventory generator | 110,111 | reproduces scene/prefab/material/animation/sprite/texture/audio/story counts; differences fail |
| KGD-114 | Source object identity and reference graph | 113 | stable logical IDs, no collisions, dangling references listed, deterministic ordering |
| KGD-115 | Reachability roots and classification model | 114 | every source object has imported, unreachable-with-proof, excluded, or blocked status; unknown is a failing state |

### Asset catalog and per-class import

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-120 | keygen.assets schema and content-addressed blob store under crates/assets/ | 003,114 | copy/translate/reimplement provenance, source/output hashes, dedupe, relative paths, bounded sizes |
| KGD-121 | Texture and flat-image importer under kg_ddlc_plus/import/images/ | 115,120 | dimensions, decoded pixel hash, color space, alpha bounds; original synthetic PNG tests |
| KGD-122 | Sprite and atlas importer under import/sprites/ | 115,120,121 | rect, pivot, border, PPU, mesh, packing, and ordering fixtures; no atlas screenshot flattening |
| KGD-123 | Font and TextMeshPro metadata importer under import/fonts/ | 115,120 | font bytes reused locally; face, glyph, kerning, line metrics, fallback, sprite-font mappings classified |
| KGD-124 | Audio importer under import/audio/ | 115,120 | codec decision, decoded PCM hash, channels, rate, duration, loop metadata; no substitutions |
| KGD-125 | Animation and animator importer under import/animation/ | 115,120 | clip bindings, keyframes, tangents, wrap, events, controller states; sampled trace fixtures |
| KGD-126 | Scene, prefab, GameObject, component, and hierarchy importer under import/scene/ | 115,120 | four scene roots and 53 prefabs classified; transform/reference hierarchy preserves stable IDs |
| KGD-127 | Camera, Canvas, RectTransform, mask, navigation, and sorting importer under import/ui/ | 123,126 | every relevant serialized field mapped or blocked; no eyeballed layout values |
| KGD-128 | Material, shader-property, and texture-binding inventory/importer under import/materials/ | 121,126 | 82 materials classified; unknown shader behavior blocks affected reachable nodes |
| KGD-129 | Localized lines, UI strings, sprites, font mappings, and bundle variants under import/locales/ | 121,123,114 | locale dependency graph complete; missing fallback is diagnostic |
| KGD-130 | Typed story, character, style, audio-table, block, and label importer under import/story/ | 114,120 | 2,160 blocks, 287 labels, 27,968 descriptors, 34 variants reconcile exactly |
| KGD-131 | Label and coarse-bundle dependency importer under import/dependencies/ | 114,130 | 82 label bundles classified; deterministic label-scoped dependency closure |
| KGD-132 | Reuse-policy checker under scripts/check-asset-reuse.* | 120–131 | fails placeholders, missing provenance, forbidden substitutions, undeclared blobs, and reachable unclassified assets |

### Package compilation

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-140 | Expanded keygen.package schema under crates/package/ | 003,120 | entry state, groups, blobs, schemas, source, compiler, options, and no undeclared files |
| KGD-141 | Deterministic reachability compiler under kg_ddlc_plus/compiler/ | 115,120,130,131,140 | two clean compiles produce byte-identical manifests/IR and same blob set |
| KGD-142 | Package validator and inventory report | 132,141 | hash, traversal, schema, reference, reuse, descriptor, capability, and source-identity failures tested |
| KGD-150 | Locale runtime catalog schema and compile projection | 129,140 | live locale dependency selection includes correct text, sprite, font, and fallback groups |

Wave 2 exits only when G0 and G1 pass. Asset counts are allowed to differ from
global source counts only through explicit unreachable or scope-excluded proof.

## Wave 3: reusable KeyGen runtime

### Retained rendering and UI

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-301 | keygen.scene.v2 schema under engine/scene/ | 103,120,127,128 | nodes, cameras, canvases, sprites, text, masks, materials, animations, focus; strict bounds and versions |
| KGD-302 | Deterministic retained scene-state reducer | 301 | create/update/remove/reparent/order operations; snapshot equality and stable traversal |
| KGD-303 | Asset/resource lifetime manager | 120,131,302 | ref counts or ownership groups; deterministic load/unload; launcher survives story group release |
| KGD-304 | Transform hierarchy and coordinate math | 301,302 | anchors, pivots, parent transforms, rotation, zoom, size, offsets; numeric edge fixtures |
| KGD-305 | Sprite sampling, alpha blend, clipping, and sorting | 121,122,304 | pixel goldens for crop/pivot/order/filter/color/alpha; bounded surfaces |
| KGD-306 | Masks, rectangular clipping, and nine-slice panels | 122,305 | edge and nested-mask goldens; recovered border semantics represented |
| KGD-307 | Font shaping and text layout abstraction | 123,301 | glyph metrics, kerning, wrapping, alignment, outline, fallback fixtures |
| KGD-308 | Animation clock, tracks, easing, parallel/repeat/event scheduler | 125,304 | fixed-clock sampled traces; cancellation and restore state |
| KGD-309 | Material/effect interface and deterministic fallback policy | 128,301 | supported properties typed; unknown reachable material cannot silently degrade |
| KGD-310 | Reference resolution, camera, aspect, Retina, and black-bar model | 127,304 | logical-to-physical matrix fixtures for qualified display modes |
| KGD-311 | Image layer and scene mutation service | 302–305 | named layers, retained show/hide/scene, child object lifetime and order |
| KGD-312 | Composited character assembly service | 122,126,311 | body/face/accessory hierarchy retained; expression swap does not flatten source |
| KGD-313 | ATL-shaped transform runtime | 308,311 | immediate/ease/time/parallel/repeat/on semantics with serialized trace |
| KGD-314 | Before/after transition compositor | 309–311 | fade/dissolve/wipe/overlay start, midpoint, end; input state unaffected |
| KGD-315 | Retained screen registry and modal stack | 301,302 | activate/deactivate, return value, prior focus restoration, nested modal rejection |
| KGD-316 | Semantic focus graph independent of drawing | 127,315 | keyboard/controller traversal, disabled nodes, default focus, modality-safe restore |
| KGD-317 | UI widget primitives | 306,307,315,316 | button, label, image, list, slider, toggle, text field, scroll view with synthetic tests |
| KGD-318 | Scene v2 integration and v1 migration | 301–317 | phase-zero BIOS migrates; full engine tests and public synthetic demo pass |

### Audio and deterministic effects

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-330 | Deterministic audio-channel state model | 103 | music, poem music, sound, launcher, and jukebox owners; play/stop/queue/fade/loop/position |
| KGD-331 | Decode/resample/buffer abstraction | 124,330 | supported source codecs decode; PCM hash/duration fixtures; bounded allocation |
| KGD-332 | Audio cancellation, snapshot, and owner handoff | 303,308,330 | load/route change cancels correct events and restores exact logical positions |

### Story program foundation

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-400 | keygen.story.v1 schema with all 34 descriptor tags | 003,130 | strict deserialize/serialize and one synthetic fixture per variant |
| KGD-401 | Typed value, scope, defaults, globals, and persistent model | 400 | bounded values, local scope stack, interpolation inputs, snapshot round trip |
| KGD-402 | Block cursor, labels, calls, jumps, and returns | 400,401 | entry points, call stack, local lifetime, invalid target diagnostics |
| KGD-403 | Conditional, line, timeout, loop, and fork scheduling | 402 | deterministic clock/input branch fixtures and fork cancellation |
| KGD-404 | Image descriptor executor | 311,400–403 | Show/Hide/Scene/LoadImage/Size queue and interaction-boundary semantics |
| KGD-405 | Transform, transition, pause, and time executor | 313,314,400–403 | With/Immediate/Ease/Time/Pause deterministic traces |
| KGD-406 | Audio descriptor executor | 330–332,400–403 | Play/Stop/Queue ownership, fade, loop, queue and restore fixtures |
| KGD-407 | Screen and dialogue descriptor protocol | 315,400–403 | Dialog/Window/WindowAuto/Text/WaitForScreen/MenuInput typed effects/results |
| KGD-408 | Expression value mutation, seeded range, unlock, clear flag, and NOP | 401–403 | repeatable randomness, typed events, no host effects in reducer |
| KGD-409 | Bounded expression parser and evaluator | 401 | supported operators/functions catalog, resource limits, unsupported syntax diagnostics |
| KGD-410 | Typed native-capability registry | 003,401 | enumerated IDs/arguments/results, no reflection or arbitrary execution, snapshot policy |
| KGD-411 | VM integration, trace, cancellation, and snapshot | 400–410 | every descriptor covered; unknown tag/capability fails; deterministic replay hash |

Wave 3 exits when G2 passes with only original synthetic public fixtures.

## Wave 4: launcher product

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-201 | Typed launcher app registry and central router under kg_ddlc_plus/runtime/launcher/ | 142,150,303,315–318,330 | only router switches; start/update/request-close/close ordering and input lock tested |
| KGD-202 | First-run, returning-user, lifecycle, and interaction-marker routing | 201 | all recovered branch combinations and story return routes replay |
| KGD-210 | BIOS importer/runtime replacement of phase-zero approximation | 121,123,124,127,201 | exact assets/layout/timing/audio; start/mid/end/skip captures; no hand-set source values |
| KGD-211 | BootUp log, spinner, audio, aspect line-count, skip | 201,210 | fixed-clock semantic and private visual captures at both aspect classes |
| KGD-212 | Login input gate, load wait, animation, desktop route | 201,211,430 | no route before store completion; focus and animation trace |
| KGD-220 | Desktop, wallpaper, clock, icons, taskbar, start menu, notifications | 121–129,201 | every recovered desktop semantic ID mapped and private reference states pass |
| KGD-221 | Shared window open/close choreography | 125,201,220 | input lock, scale/alpha/position, fade panel, taskbar, focus timing trace |
| KGD-222 | Confirmation, error, quit, and save-busy modals | 221 | nested behavior, cancel/confirm, focus restore, active-save guard |
| KGD-230 | File Browser | 220–222,440 | hierarchy, selection, folder rows, open/delete/reset gates |
| KGD-231 | File Viewer | 230,331 | text/sprite/audio modes and return handoff |
| KGD-232 | Mail | 220–222,434,441 | browse/read, last-viewed, unlock notification, locale refresh |
| KGD-233 | Side Stories | 220–222,240,441 | unlock projection, confirm, launch parameters, return |
| KGD-234 | Gallery | 220–222,441 | categories, locked/unlocked projection, full image, locale assets |
| KGD-235 | Jukebox and mini-player | 220–222,331,332,434 | browse/play/pause/queue, persistent state, route ownership |
| KGD-236 | Launcher Settings | 220–222,435,442 | every included preference maps, saves on close, live effects and locale refresh |
| KGD-237 | VM terminal/glitch/reset sequence | 220–222,314,330,440,441 | deterministic seeded trace, audio/effects/unlock/reset routes |
| KGD-240 | Launcher-to-story handoff and resource ownership | 201,303,332,411 | immutable parameters, launcher suspend, story start, teardown, resource/audio/focus restore |

Wave 4 exits when G3 passes for both first-run and returning-user routes and
every launcher app has semantic, state, and private capture evidence.

## Wave 5: VN shell and imported interpreter

### Native screen set

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-320 | Dialogue window, styles, speaker focus, CTC, quick controls | 307,312,315–318,407 | recovered panels/fonts/styles/focus maps reused and captured |
| KGD-321 | Typewriter, punctuation, dismiss, skip, auto-forward | 320,435 | reference cadence measured; fixed-clock reveal, skip and auto fixtures |
| KGD-322 | Choice screen and conditional navigation | 316,317,407 | disabled/conditional entries, default focus, mouse/controller result |
| KGD-323 | DDLC title MainMenu and Navigation screens | 240,315–318,330 | exact recovered UI assets/layout/audio; all actions and return paths |
| KGD-324 | History screen | 320,433 | encountered lines, scrolling, styles, locale and empty state |
| KGD-325 | Save and Load screens | 317,430,431 | 54 slots, paging, screenshot, empty/corrupt state, confirm, restore |
| KGD-326 | VN Preferences screen | 317,435 | all included story preferences and immediate projections |
| KGD-327 | Name input screen | 317,500,435 | validation, cancel/accept, normalized text/action input; macOS adapter is qualified later |
| KGD-328 | Poetry game, instructions, word selection, poem display | 312,317,331,408,442 | localized words, affinities, assets, audio and navigation replay |
| KGD-329 | Confirm, quick menu, tear, invert, account and controller fallback screens | 314–317,500 | each recovered screen ID classified and reachable states captured |

### DDLC story import and capability closure

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-412 | OneLinePython expression mapping inventory | 130,409 | every reachable expression maps to supported AST or a blocking diagnostic |
| KGD-413 | Function and InlinePython capability identity inventory | 130,410 | every reachable call classified by domain, signature, side effects, save policy |
| KGD-414 | Capability batch generator under tasks/generated/ | 413 | emits stable batches of at most eight cohesive handlers with dependencies and tests |
| KGD-415 | Core variable, flow, interpolation, and transform capability batches | 414 | generated batch manifests green; deterministic effect traces |
| KGD-416 | Dialogue, screen, poem, menu, and special-effect capability batches | 320–329,414 | generated batch manifests green; screen result and save policies tested |
| KGD-417 | File, persistence, unlock, and launcher-bridge capability batches | 240,414,430–435,440–441 | sandbox and state effects typed; no arbitrary filesystem calls |
| KGD-418 | Character and story-special-case capability batches | 312,414 | every reachable handler implemented or scope-blocking evidence filed |
| KGD-419 | Imported story validation and reachability pass | 412–418 | 2,160 blocks, 287 labels, 27,968 descriptors reconcile; zero unknown reachable call |
| KGD-420 | Deterministic story replay harness | 320–329,419 | scripted inputs produce stable state, frame, audio-event, and store-effect hashes |

KGD-414's generated packets become ledger children KGD-415.x through KGD-418.x.
They may run in parallel only when their generated writable paths do not
overlap. The generated catalog, not a guessed manual list, makes capability
coverage exhaustive.

Wave 5 exits when G4 passes and every imported descriptor/expression/capability
has a machine-checked implementation or an explicit scope-blocking diagnostic.

## Wave 6: content, persistence, and localization

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-421 | Character pose/expression assembly coverage | 312,419 | every reachable named image resolves to reused layers; representative private captures |
| KGD-422 | Background, CG, GUI, and special-image coverage | 311,419 | all reachable logical images reused and dependency-scoped |
| KGD-423 | Animation, material, and transition coverage | 313,314,419 | every reachable clip/material/transition classified and reference-tested |
| KGD-424 | Audio alias, clip, and dependency coverage | 331,332,419 | every reachable alias resolves to reused clip and correct channel/loop metadata |
| KGD-425 | Label-scoped resource reconciliation | 303,131,419,421–424 | reachable closure complete; load/unload trace; desktop survives all story cycles |
| KGD-430 | Versioned atomic store framework under player/storage/ | 003,102 | checksums, temp write/fsync/rename, corruption recovery, migrations, write exclusion |
| KGD-431 | Runtime slot, autosave, lifecycle, and 384×216 screenshot state | 411,430 | cursor/stacks/scopes/images/transforms/screens/history/audio round trip |
| KGD-432 | Persistent story-variable store | 401,430 | independent versioning and atomic save; reset semantics |
| KGD-433 | Seen-text and history store | 320,430 | skip/read projection and bounded history round trip |
| KGD-434 | Launcher and launcher-lifecycle stores | 201,430 | every app state, wallpaper, unlock and jukebox projection; one-shot restore |
| KGD-435 | Preference store | 150,330,430 | defaults, migration and validation; screen/typewriter/audio live projection tested by consumers |
| KGD-440 | Sandboxed virtual filesystem | 102,430 | allow-listed logical paths, visibility, timestamps, deletion policy, stale cleanup; traversal fails |
| KGD-441 | Local unlock/progression and null external achievement provider | 408,430 | deterministic events, app/story projection, reset; no Steam dependency |
| KGD-442 | Localization runtime and live reload | 150,307,435 | locale switch atomically selects text/sprite/font groups; complete content coverage is enforced by KGD-444 |
| KGD-443 | Save-loading scene and failure/recovery UX | 222,325,431–435 | busy, corrupt, incompatible, restored, and failed states; user retains last valid save |
| KGD-444 | Full reachable-content compile | 419,421–425,430–435,440–443 | zero unclassified reachable object, asset, descriptor, expression, or capability |
| KGD-445 | Story completion and progression suite | 420,444 | deterministic route suite covers endings, side stories, unlocks, reset and return-to-launcher |

Wave 6 exits only when G5 and G6 pass. Content volume does not allow a
lower-quality substitution; coverage is generated from the recovered graph.

## Wave 7: Apple Silicon macOS application

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-500 | Platform-neutral action/input event model | 103,316 | key, pointer, text, controller, focus, connect/disconnect replay |
| KGD-501 | macOS keyboard, mouse, text, cursor, and focus adapter | 101,500 | native window test, key repeat, pointer scaling, text input, activation focus |
| KGD-502 | macOS controller and glyph-modality adapter | 101,500 | connect/disconnect, axis/button normalization, modality refresh |
| KGD-503 | macOS audio output adapter | 101,331,332 | device open, stable buffer, volume/fade, pause/resume, ownership handoff |
| KGD-504 | Metal renderer and upload path | 101,301–318 | correct alpha/color/scale, shader family support, frame capture, device-loss diagnostic |
| KGD-510 | Native macOS window and event loop | 101,500,504 | arm64, resizable native window, no WebView/browser/network dependency |
| KGD-511 | Renderer, input, audio, and lifecycle host integration | 501–504,510 | same runtime state drives headless and native output; clean launch/run/quit |
| KGD-512 | Windowed, fullscreen, Retina, aspect, and black-bar behavior | 310,511 | qualified display matrix and pointer mapping pass |
| KGD-520 | macOS app activation, deactivation, low-memory, and termination lifecycle | 101,430,511 | lifecycle snapshot order, audio pause/resume, no torn write |
| KGD-521 | Quit confirmation and active-save interception | 222,520 | close button, menu quit, cancel, confirm, busy-save paths never crash |
| KGD-522 | Application Support, cache, log, screenshot, and save paths | 102,430,520 | distinct sandbox paths, redacted diagnostics, no official path write |
| KGD-530 | Deterministic arm64 app-bundle builder under scripts/package-macos.* | 102,142,511,522 | exact bundle structure, declared package only, reproducible metadata |
| KGD-531 | KeyGen-original icon and Info.plist metadata | 102,530 | Finder/Spotlight name, arm64, bundle ID, minimum OS, copyright distinction |
| KGD-532 | Validation, ad-hoc signing, exact-target installer, and launcher | 530,531 | codesign verify, staged validation, install only /Applications/kg_ddlc_plus.app |
| KGD-533 | Canonical-install and duplicate audit | 532 | one canonical installed target; reports but never deletes unrelated or temp bundles |

Wave 7 exits when G7 passes on the qualification Mac. A raw cargo window is not
an application-completion substitute.

## Wave 8: private conformance and release qualification

| ID | Deliverable and path family | Depends | Acceptance |
| --- | --- | --- | --- |
| KGD-600 | Reference-state catalog generated from scope matrix | 004,005 | every required state has build/locale/display/input/time/source IDs |
| KGD-601 | Read-only private capture runner for official app | 005,600 | reproducible screenshot/audio/timing/action evidence; no input mutation beyond test profile |
| KGD-602 | KeyGen scripted replay and capture runner | 420,511,600 | same normalized actions/timestamps emit semantic, frame, audio, and state evidence |
| KGD-603 | Measured startup, frame, memory, package, and cycle budgets | 104,601,602 | thresholds derived and recorded from reference and qualification hardware |
| KGD-604 | Visual comparator by primitive and state | 601,602 | dimensions/alignment/color/alpha plus masked per-primitive tolerances; heatmap stays private |
| KGD-605 | Animation and transition trace comparator | 601,602 | start/mid/end and sampled tracks meet measured timing/geometry tolerances |
| KGD-606 | Audio metadata, event, duration, and decoded-sample comparator | 601,602 | alias/channel/timing/PCM comparisons with explicit codec tolerance |
| KGD-607 | Semantic route, focus, input-lock, store-effect, and lifecycle comparator | 601,602 | every matrix action produces expected state and effects |
| KGD-608 | Asset reuse and reachability final audit | 132,142,444 | zero substitute, undeclared, missing, unknown, or unclassified reachable item |
| KGD-609 | Clean-profile end-to-end playthrough and progression suite | 445,532,603–608 | first-run through completion/reset/side stories and returning route passes |
| KGD-610 | Repeated launch, desktop/story cycles, save interruption, corruption, and leak soak | 609 | budgets pass across recorded repetition count; last valid state always recoverable |
| KGD-611 | Final G0–G8 release gate and operator report | 608–610,533 | all gates green, clean Git, ignored content proof, installed app searchable and launchable |

## Required commands at every integration

~~~sh
git diff --check
python3 sz.py
scripts/check-fast.sh
scripts/check-full.sh
cargo run -p kg-ddlc-plus -- inspect
cargo run -p kg-ddlc-plus -- compile
cargo run -p kg-ddlc-plus -- validate
~~~

After KGD-001 and later gate packets exist, their generated scope, privacy,
reuse, coverage, and app-validation commands join check-fast or check-full.
Private conformance commands run locally and report only aggregate pass/fail
evidence into Git.

## Definition of exhaustive

The ledger is exhaustive when all of the following are simultaneously true:

1. Every row in the canonical traceability matrix points to one or more closed
   packets.
2. Every reachable source object is classified by KGD-115 and KGD-608.
3. Every asset has reuse provenance and no available asset was substituted.
4. Every one of the 34 descriptor variants has parser, VM, and replay evidence.
5. Every reachable expression and native capability is green in the generated
   KGD-412 through KGD-418 catalogs.
6. Every launcher app, VN screen, state store, locale, input mode, audio owner,
   and lifecycle route has reference evidence.
7. Exactly one validated arm64 kg_ddlc_plus.app is installed canonically.
8. G0 through G8 are green and the full repository hooks pass.

No fixed hand-written task list can honestly predict every data-dependent
capability before import. The generated inventory and zero-unclassified gates
make that unknown finite, visible, automatically partitioned, and blocking.
