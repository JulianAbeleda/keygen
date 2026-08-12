# Generic KeyGen macOS boundary

The macOS application boundary is target-neutral. `scripts/package-keygen-macos.sh`
accepts a compiled arm64 host plus target-supplied identity and resources, then
creates a deterministic `Contents/MacOS`, `Info.plist`, and package manifest.
No DDLC names, Steam identifiers, story labels, or proprietary assets are
embedded in the builder. `kg_ddlc_plus` therefore remains one adapter among
future compatibility targets and can migrate from its legacy wrapper without
changing the KeyGen host contract.

Validation is fail-closed for non-macOS, non-arm64 hosts and malformed target or
bundle identifiers. Resource ownership remains with the adapter and the bundle
manifest records hashes rather than source provenance or absolute paths.
