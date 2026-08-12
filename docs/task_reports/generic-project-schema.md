# Generic project schema

KeyGen now exposes `keygen_engine::project::ProjectManifest` (`keygen.project.v1`).
It is an editor-free, title-neutral package contract for project identity, viewport,
content-addressed logical assets, scenes, story entry labels, and persistence
namespace. Hosts can load JSON from bytes or a path and receive deterministic
validation for duplicate IDs, missing scene assets, malformed hashes, and invalid
story entry points.

The compatibility target may compile into this schema, but the engine does not
depend on or name that target. New games can provide the same manifest without
Unity, Ren'Py, browser JavaScript, or a game-specific engine fork.
