# Generic native launch boundary

This slice adds the product-neutral launch and lifecycle contract consumed by
a future AppKit/Metal host. `HostLaunchSpec` carries target identity, initial
route, and session-restore policy; it does not contain DDLC-specific values.
`HostLifecycle` enforces the save barrier before quit/deactivation transitions.

The generic macOS bundle now advertises arm64 launch metadata, high-resolution
capability, and the AppKit principal class. This is bundle metadata only: the
current Rust presentation backend remains the existing test/minifb backend.
No AppKit, Metal, CoreAudio, or unsafe FFI is introduced by this packet.

Qualification coverage is provided by unit tests for launch identity validation,
save-before-quit ordering, and the existing backend lifecycle state machine.
