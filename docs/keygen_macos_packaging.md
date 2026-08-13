# Generic KeyGen macOS packaging

KeyGen owns the reusable engine and host boundary. A compatibility target
supplies an identity, compiled host binary, and optional content package. The
generic builder does not inspect or name DDLC or any other target.

```sh
scripts/package-keygen-macos.sh --binary target/release/my-target \
  --target my_target --display-name "My Target" \
  --bundle-id com.example.my-target --resources local/my_target
```

The result is `dist/macos/my_target.app`: an arm64 executable, `Info.plist`,
optional resources, and a SHA-256 package manifest. It requires Apple Silicon
macOS and refuses malformed identifiers. `kg_ddlc_plus` remains an adapter;
its identity, importer, story descriptors, and assets are package-owned.

Finder/LaunchServices starts the executable without arguments. The package
contract therefore reserves `Resources/package/project.json` and
`Resources/package/scenes/boot.json` as the canonical no-argument launch path;
the generic binary validates that manifest before opening the scene.
