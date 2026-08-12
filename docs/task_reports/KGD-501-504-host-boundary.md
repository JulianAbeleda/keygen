# KGD-501–504 — Native presentation boundary

The player now exposes a backend-neutral presentation contract in
`keygen_player::native`:

- `DesignViewport` fixes the logical coordinate space shared by captures and
  native windows.
- `HostFrame` validates owned RGBA8 payloads before they reach a window.
- `PresentationBackend` separates lifecycle/presentation from the current
  minifb loop, allowing an AppKit/Metal adapter to consume the same frames.
- `AudioBackend` and `AudioCommand` keep logical audio effects separate from
  CoreAudio scheduling.

The in-memory test backend proves frame rejection after close and deterministic
lifecycle behavior without opening a window. This is an integration boundary,
not a claim that AppKit, Metal, CoreAudio, signing, or text-input bridges are
implemented. Those adapters remain platform work items.
