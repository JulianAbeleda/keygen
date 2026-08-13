# Native host adapter qualification

The player now has a concrete macOS system-audio adapter in
`crates/player/src/native.rs`. Logical `AudioCommand::Play` clips are resolved
by the package manifest and submitted to the shipped `afplay` client, which
uses the macOS CoreAudio stack. Channels are independently stopped and all
children are terminated on adapter drop. No engine or product code invokes a
platform path directly.

The existing interactive window remains the minifb backend. On macOS this is a
native Cocoa window, but it is not a Metal renderer. `native_capabilities()`
reports this boundary explicitly: AppKit window and CoreAudio are available on
macOS; Metal and native text input remain false until their adapters are
implemented. This avoids packaging or qualification claiming a capability we
do not provide.

Qualification coverage includes registration/existence checks, idempotent
stop behavior, capability honesty, lifecycle retention, and unsupported-host
launch rejection. The adapter is intentionally safe Rust and does not add an
unsafe FFI dependency.
