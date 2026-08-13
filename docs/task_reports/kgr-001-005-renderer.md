# KGR-001–KGR-005 renderer substrate

Implemented the reusable polygon coverage substrate in `keygen-engine`.
`Canvas::polygon` remains source-compatible and now defaults to deterministic
4×4 coverage sampling. `polygon_with_options` exposes `FillRule` (even-odd or
winding), `AntialiasMode` (coverage or binary), and `FillOptions`.

The mask is evaluated only over the polygon's clipped bounding box. Non-finite
vertices and fewer than three vertices fail closed. Coverage is passed as
opacity to the existing source-over compositor, preserving opaque RGBA8
surface semantics. Added tests cover fractional diagonal coverage,
determinism, non-finite and degenerate input, concave shapes, even-odd versus
winding behavior, vertex-order invariance, binary fallback, and logical
bounds across density changes.

Coverage now uses four horizontal scanlines, sorted edge crossings, and
per-row coverage counts. Release timing evidence from the representative
1600x900@2x quadrilateral is 207.2 ms for 10 draws (about 20.7 ms/draw).
That isolated primitive is below one 33.3 ms frame budget, but consumer-level
acceptance still requires timing the complete GameTerm frame after rollout.
The timing test is ignored by default and
reproducible with `cargo test -p keygen-engine polygon --release -- --ignored
--nocapture`.

Checks: `cargo fmt --all`; `cargo test -p keygen-engine -p keygen-player`
(30 engine tests passed, 1 timing test ignored, and 31 player tests passed);
`cargo clippy --workspace --all-targets -- -D warnings`.

The generic host now also exposes `render_frame_scaled`, so products can
produce deterministic physical-resolution evidence without a display. The
existing `render_frame` remains source-compatible and delegates at density 1.

Risk: fixed 4×4 sampling is a compact deterministic fallback rather than an
analytic tiny-skia mask; very thin features can vary with sample placement.
The API leaves room to replace the implementation without changing callers.

Also added `Canvas::blit_surface`, a clipped exact physical-coordinate copy
for opaque RGBA8 surfaces. It rejects source alpha other than 255 and covers
clipping, density/backing coordinates, determinism, and alpha rejection.
