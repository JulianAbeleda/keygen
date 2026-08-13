# Generic route navigation

`keygen-engine` now exposes `ProjectRouteNavigator`, a small host-facing
controller for the `keygen.project.v1` route table. It makes the packaged
application sequence explicit:

```text
Boot --advance_boot--> Launcher --activate(route id)--> Story or App
Story/App --back--> Launcher --back--> Boot
```

Route IDs, scene IDs, and optional story labels come from the validated project
manifest. The native host can therefore translate keyboard, pointer, or menu
events into `advance_boot`, `activate`, and `back` without inspecting asset
filenames or embedding a title-specific route table. `close` produces an
explicit `Closed` route for host lifecycle handling.

This is navigation state only: presentation, audio, persistence, and platform
window ownership remain host adapters. The implementation does not claim a
Metal backend and does not read private source assets.
