# KGD-101: arm64 macOS product target

The `kg_ddlc_plus` product target is Apple Silicon macOS (`arm64`) only. Its
deployment floor is recorded as macOS 15.0 while the native host and rendering
dependencies are qualified. Intel, universal, Linux, Windows, mobile, and
console product binaries are out of scope.

General KeyGen source-health CI may remain multi-platform; that does not create
additional `kg_ddlc_plus` products. The native app must report its architecture
and reject a non-arm64 packaging request before writing output.

The floor is intentionally an explicit decision rather than an inferred host
version. It can be revised by a future decision record after qualification,
never implicitly by a feature packet.
