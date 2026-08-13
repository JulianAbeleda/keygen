# Native macOS adapter slice

This slice adds a safe host-facing macOS adapter boundary without claiming an
AppKit, Metal, or CoreAudio implementation. `MacOsLaunchAdapter` validates a
`.app` layout and resolves its executable without shelling out or starting a
process. `MacOsQualificationBackend` consumes the same owned RGBA frames and
lifecycle events that a future AppKit/Metal backend will consume, retaining the
latest frame for deterministic host qualification.

The current interactive player remains minifb-backed. The adapter is
intentionally safe Rust (`keygen-player` forbids unsafe code), so Cocoa/Metal
FFI is not introduced as a disguised partial implementation. A future native
backend can implement `PresentationBackend` and preserve the engine/story
contracts and launch adapter unchanged.

Validation includes frame rejection after close, latest-frame retention, and
malformed bundle rejection. This is an integration boundary, not a claim that
native rendering/audio/input are complete.
