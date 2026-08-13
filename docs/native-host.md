# Native application host

`keygen-player::host` is a title-neutral host for products that need a native
window without duplicating input, timing, or presentation code. Implement
`Application::frame` and optionally `Application::event`, then call
`host::run`. `host::render_frame` provides the same contract for headless
captures and tests.

Drawing is performed with `keygen_engine::Canvas`, an immediate-mode facade
over the deterministic RGBA `Surface`. It supports decoded PNG images,
contain/cover/stretch composition, rectangles, rounded rectangles, polygons,
and fontdue text with optional outline. `Surface::encode_png` is stable for a
given canvas and is suitable for golden-image tests.

The host reports monotonic elapsed time and normalized tick, keyboard,
printable-text, pointer, and close events. Window size, title, resizability,
and borderless/fullscreen policy are supplied by `WindowPolicy`. The current
backend is minifb; products should not depend on that implementation detail.
