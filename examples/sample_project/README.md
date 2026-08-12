# KeyGen sample project

This deliberately generic, non-DDLC fixture exercises the editor-free KeyGen
project boundary. It contains no game-specific or proprietary assets.

From the repository root, run `examples/sample_project/smoke.sh`. The script
creates deterministic synthetic PNG data, validates `project.json`, loads the
scene, and renders `out/sample.png`. A macOS system TrueType font is used at
runtime so no font is redistributed.
