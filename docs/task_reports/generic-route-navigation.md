# Generic route navigation

`keygen-engine` now exposes `ProjectRouteNavigator`, a small host-facing
controller for the `keygen.project.v1` route table. It makes the packaged
application sequence explicit:

```text
Boot --advance_boot--> Launcher --activate(route id)--> Story or App
Story/App --back--> Launcher --back--> Boot
```

The player’s native scene loop discovers the adjacent `project.json` when a
scene is launched from a packaged `Resources/package/scenes` directory. It
advances the navigator after boot and sends activated menu IDs through the
same route table, logging the resolved story/app transition. Bundles without a
project manifest retain the legacy scene-only behavior.

Route IDs, scene IDs, and optional story labels come from the validated project
manifest. The native host can therefore translate keyboard, pointer, or menu
events into `advance_boot`, `activate`, and `back` without inspecting asset
filenames or embedding a title-specific route table. `close` produces an
explicit `Closed` route for host lifecycle handling.

This is navigation state only: presentation, audio, persistence, and platform
window ownership remain host adapters. The implementation does not claim a
Metal backend and does not read private source assets. The native player now
discovers the adjacent project manifest for packaged scenes and feeds menu
activations and Escape/exit into this navigator; bundles without a project
manifest retain the legacy scene-only behavior.
